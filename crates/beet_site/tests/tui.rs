//! Scenario 1 (and the live-TUI harness): navigation, scrolling, links, driven
//! against `beet_site`'s real router through an in-process `ChannelTerminal`.
//!
//! Mirrors `main.rs`'s `tui` wiring (the live plugins + `beet_site_router`) but
//! swaps the `StdioTerminal` for a `ChannelTerminal`, so it is headless and
//! deterministic. The behavioral gate the binary cannot be in a sandbox.
beet::test_main!();

use beet::prelude::*;
use beet_site::prelude::*;

/// Records every [`OpenExternalLink`] (an external-open intent), the test seam
/// over the real system-browser launch.
#[derive(Resource, Default)]
struct ExternalOpens(Vec<Url>);

/// A booted live-TUI site app: the real router + a page host + an in-world
/// navigator, driven through a channel terminal.
struct SiteHost {
	app: App,
	host: Entity,
	nav: Entity,
}

/// An SGR mouse sequence: button `b` at 0-indexed cell `(col, row)`, pressed
/// (`M`) or released (`m`).
fn sgr(b: u32, col: u32, row: u32, pressed: bool) -> Vec<u8> {
	let m = if pressed { 'M' } else { 'm' };
	format!("\x1b[<{b};{};{}{m}", col + 1, row + 1).into_bytes()
}

impl SiteHost {
	/// Boot the site at `home` with a `size`-cell viewport.
	fn new(size: UVec2, home: &str) -> Self {
		Self::new_with(size, home, |_| {})
	}

	/// Boot the site at `home`, running `setup` before the first frame — for
	/// app state that must exist when the initial page renders (eg the
	/// session [`ColorScheme`], which the binary seeds before navigating).
	fn new_with(
		size: UVec2,
		home: &str,
		setup: impl FnOnce(&mut App),
	) -> Self {
		let mut app = App::new();
		// the shared site substrate (router + style rules + package config) plus the
		// live interactive plugins, exactly as `main.rs`'s `tui` arm composes them.
		app.add_plugins(server_plugin)
			.insert_resource(PackageConfig {
				title: "Beet".to_string(),
				..pkg_config!()
			})
			.add_plugins((
				CharcellTuiPlugin,
				NavigatorPlugin,
				LivePagePlugin,
				FormRuntimePlugin,
			));
		// deterministic frames whatever terminal runs the tests: no kitty
		// escapes unless a test opts back in via `setup`.
		app.insert_resource(KittyGraphicsSupport { enabled: false });
		setup(&mut app);
		// intercept external-open intents instead of launching a browser.
		app.init_resource::<ExternalOpens>();
		app.add_systems(
			Update,
			|mut events: MessageReader<OpenExternalLink>,
			 mut opens: ResMut<ExternalOpens>| {
				for ev in events.read() {
					opens.0.push(ev.url.clone());
				}
			},
		);

		// the router on its own entity; the in-world navigator dispatches to it.
		let router = app.world_mut().spawn(beet_site_router()).id();
		// the host pairs a channel terminal with the page-host buffer.
		let (channel, terminal) = ChannelTerminal::new(TerminalConfig::default());
		let host =
			app.world_mut().spawn((channel, terminal, page_host(size))).id();
		let nav = app.world_mut().spawn(Navigator::in_world(router, home)).id();
		app.update();
		Self { app, host, nav }
	}

	/// Push raw input bytes (keys, SGR mouse) into the channel terminal.
	fn send(&mut self, data: &[u8]) {
		self.app
			.world_mut()
			.get_mut::<ChannelTerminal>(self.host)
			.unwrap()
			.send_input(data)
			.unwrap();
	}

	/// The 0-indexed start cell of the first `text` occurrence in the frame.
	fn cell_of(&self, text: &str) -> (u32, u32) {
		for (row, line) in self.frame().lines().enumerate() {
			if let Some(col) = line.find(text) {
				return (col as u32, row as u32);
			}
		}
		panic!("text {text:?} not found in frame:\n{}", self.frame());
	}

	/// Click (press + release) the cell at `(col, row)`.
	fn click(&mut self, col: u32, row: u32) {
		self.send(&sgr(0, col, row, true));
		self.app.update();
		self.send(&sgr(0, col, row, false));
		self.app.update();
	}

	/// Queue an in-world navigation to `path`.
	fn navigate(&mut self, path: &str) {
		let url = Url::parse(path);
		self.app
			.world_mut()
			.entity_mut(self.nav)
			.run_async_local(move |entity| Navigator::navigate_to(entity, url));
	}

	/// Resize the host buffer, as a terminal resize would.
	fn resize(&mut self, size: UVec2) {
		self.app
			.world_mut()
			.get_mut::<DoubleBuffer>(self.host)
			.unwrap()
			.resize(size);
	}

	/// Move the cursor over cell `(col, row)` (an SGR motion event) to set hover.
	fn hover(&mut self, col: u32, row: u32) {
		self.send(format!("\x1b[<35;{};{}M", col + 1, row + 1).as_bytes());
		self.app.update();
	}

	/// A wheel-down event over cell `(col, row)`.
	fn wheel_down(&mut self, col: u32, row: u32) {
		self.send(format!("\x1b[<65;{};{}M", col + 1, row + 1).as_bytes());
		self.app.update();
	}

	/// The `name` attribute of the currently focused element, if it has one.
	fn focused_name(&mut self) -> Option<String> {
		self.app
			.world_mut()
			.run_system_once(
				|focused: Query<Entity, With<Focus>>, elements: ElementQuery| {
					let entity = focused.single().ok()?;
					let name = elements.get(entity).ok()?.attribute_string("name");
					(!name.is_empty()).then_some(name)
				},
			)
			.ok()
			.flatten()
	}

	/// Whether any rendered element carries the given class.
	fn has_class(&mut self, class: &str) -> bool {
		let class = class.to_string();
		self.app
			.world_mut()
			.run_system_once(move |elements: ElementQuery| {
				elements.iter().any(|view| view.contains_class(&class))
			})
			.unwrap_or(false)
	}

	/// A clickable center cell of the element whose `name` attribute is `name`.
	fn element_cell(&mut self, name: &str) -> (u32, u32) {
		let name = name.to_string();
		let rect = self
			.app
			.world_mut()
			.run_system_once(
				move |elements: ElementQuery, rects: Query<&LayoutRect>| {
					elements
						.iter()
						.find(|view| view.attribute_string("name") == name)
						.and_then(|view| rects.get(view.entity).ok().copied())
				},
			)
			.ok()
			.flatten()
			.expect("a laid-out element with that name")
			.0;
		(
			((rect.min.x + rect.max.x) / 2) as u32,
			((rect.min.y + rect.max.y) / 2) as u32,
		)
	}

	/// The vertical scrollbar's column and its track + thumb rows, scanned from the
	/// painted buffer.
	fn vbar(&self) -> (u32, Vec<u32>, Vec<u32>) {
		let dbuf = self.app.world().get::<DoubleBuffer>(self.host).unwrap();
		let bar: Vec<(u32, u32, String)> = dbuf
			.front_buffer()
			.iter_cells()
			.filter(|(_, cell)| matches!(cell.symbol_str(), "│" | "█"))
			.map(|(pos, cell)| (pos.x, pos.y, cell.symbol_str().to_string()))
			.collect();
		let col = bar.iter().map(|(x, _, _)| *x).max().expect("a vertical bar");
		let mut track: Vec<u32> =
			bar.iter().filter(|(x, _, _)| *x == col).map(|(_, y, _)| *y).collect();
		let mut thumb: Vec<u32> = bar
			.iter()
			.filter(|(x, _, glyph)| *x == col && glyph == "█")
			.map(|(_, y, _)| *y)
			.collect();
		track.sort();
		thumb.sort();
		(col, track, thumb)
	}

	/// The painted frame as plain text.
	fn frame(&self) -> String {
		self.app
			.world()
			.get::<DoubleBuffer>(self.host)
			.unwrap()
			.front_buffer()
			.render_plain()
	}

	/// Advance frames until the frame contains `needle`, returning the frame.
	fn step_until(&mut self, needle: &str) -> String {
		for _ in 0..200 {
			self.app.update();
			let frame = self.frame();
			if frame.contains(needle) {
				return frame;
			}
		}
		panic!("frame never contained '{needle}':\n{}", self.frame());
	}

	/// Whether a scrollbar is painted. Detected by the thumb glyph, which (unlike
	/// the `│`/`─` track) is unambiguous against card/box borders.
	fn has_scrollbar(&self) -> bool {
		self.app
			.world()
			.get::<DoubleBuffer>(self.host)
			.unwrap()
			.front_buffer()
			.iter_cells()
			.any(|(_, cell)| matches!(cell.symbol_str(), "█" | "▐" | "▄"))
	}
}

/// The homepage boots and renders, and (being short) shows no scrollbar.
#[beet::test]
async fn homepage_boots_with_chrome_and_scheme() {
	// a viewport tall enough for the full homepage + document chrome to fit.
	let mut host = SiteHost::new(UVec2::new(120, 96), "/");
	host.step_until("malleable application framework");
	// the BaseLayout chrome renders: header nav, sidebar, footer.
	let frame = host.frame();
	frame.as_str().xpect_contains("Docs").xpect_contains("Blog");
	frame.xpect_contains("© Beet");
	// the terminal target seeds the dark scheme (no web color-scheme script).
	host.has_class("dark-scheme").xpect_true();
	// everything fits this viewport, so the `auto` scrollport shows no bar.
	host.has_scrollbar().xpect_false();
}

/// Alt+Left / Alt+Right drive the navigator back / forward through history.
#[beet::test]
async fn alt_arrow_navigates_history() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/");
	host.step_until("malleable application framework");
	host.navigate("/blog");
	host.step_until("Full Moon Harvest");
	// alt+left -> back to the homepage
	host.send(b"\x1b[1;3D");
	host.step_until("malleable application framework");
	// alt+right -> forward to the blog index
	host.send(b"\x1b[1;3C");
	host.step_until("Full Moon Harvest");
}

/// In-world navigation reaches the blog index (a markdown `BlobScene` route).
#[beet::test]
async fn navigates_to_blog_index() {
	let mut host = SiteHost::new(UVec2::new(80, 40), "/");
	host.step_until("malleable application framework");
	host.navigate("/blog");
	host.step_until("Full Moon Harvest");
}

/// Scenario 1's heart: click a blog-post link, confirm navigation + a scrollbar,
/// then scroll the post both by input (wheel) and by the scrollbar itself.
#[beet::test]
async fn post_navigation_and_scrolling() {
	// a small viewport so the long post overflows and gets a scrollbar.
	let mut host = SiteHost::new(UVec2::new(60, 20), "/");
	host.navigate("/blog");
	host.step_until("Gentle Slopes");

	// click the #12 post heading link -> navigate to the post.
	let (col, row) = host.cell_of("Gentle Slopes");
	host.click(col, row);
	// navigation happened: the post body shows, the index's post blurbs are gone.
	host.step_until("Wollongong");
	host.frame().xnot().xpect_contains("mountain peak of specialization");
	// the long post overflows the viewport, so a scrollbar exists.
	host.has_scrollbar().xpect_true();

	// scroll by input: hover the content, then wheel-down advances the rows.
	let before = host.frame();
	host.hover(20, 10);
	for _ in 0..4 {
		host.wheel_down(20, 10);
	}
	let after_wheel = host.frame();
	(before != after_wheel).xpect_true();

	// scroll by the scrollbar: click the track below the thumb to page further.
	let (bar_col, track, thumb) = host.vbar();
	let below_thumb = *thumb.last().unwrap() + 1;
	(track.contains(&below_thumb)).xpect_true();
	host.click(bar_col, below_thumb);
	host.app.update();
	(host.frame() != after_wheel).xpect_true();
}

/// On `docs/references`, clicking an external link does not navigate the TUI; it
/// fires the external-open path instead.
#[beet::test]
async fn external_link_does_not_navigate() {
	let mut host = SiteHost::new(UVec2::new(80, 40), "/docs/references");
	host.step_until("References");
	let references = host.frame();

	// the first external link on the page (a `youtube` link to an absolute URL).
	let (col, row) = host.cell_of("youtube");
	host.click(col, row);
	host.app.update();

	// the external-open intent fired with an absolute URL.
	let opens = &host.app.world().resource::<ExternalOpens>().0;
	opens.is_empty().xpect_false();
	opens[0].is_external().xpect_true();
	// and the TUI did not navigate: the references page still shows.
	host.frame().xpect_contains("References");
	host.frame().xpect_contains("Malleable software");
	// the page is unchanged by the external click.
	(host.frame() == references).xpect_true();
}

/// The form page's `<select>` is a working dropdown: the closed control shows
/// its default option's label, clicking it opens the option panel, choosing a
/// row writes the value, closes the panel, and re-labels the control.
#[beet::test]
async fn form_select_opens_and_chooses() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/docs/design/form");
	host.step_until("Submit");
	// closed: the first option's label plus the dropdown caret
	host.frame().xpect_contains("Engineer ▾");
	host.frame().xnot().xpect_contains("Designer");

	// click the select: the panel opens with every option row
	let (col, row) = host.element_cell("role");
	host.click(col, row);
	let frame = host.step_until("Teacher");
	frame.xpect_contains("Designer");

	// click the Designer row: chosen, closed, the control re-labels
	let (col, row) = host.cell_of("Designer");
	host.click(col, row);
	host.step_until("Designer ▾");
	host.frame().xnot().xpect_contains("Teacher");

	// the chosen value submits with the form
	let (col, row) = host.cell_of("Submit");
	host.click(col, row);
	host.step_until("\"role\": \"designer\"");
}

/// Resizing while on the crates index (which has wide markdown tables) does not
/// panic across extreme widths/heights and a non-zero scroll offset.
#[beet::test]
async fn resize_on_crates_page_does_not_panic() {
	let mut host = SiteHost::new(UVec2::new(120, 40), "/docs/crates");
	host.step_until("Crates");
	// scroll into the body so resize happens with a non-zero scroll offset.
	host.hover(60, 20);
	for _ in 0..6 {
		host.wheel_down(60, 20);
	}
	// shrink narrower than the table columns, grow, then extremes — each reflow
	// must not panic.
	for size in [(60, 20), (8, 6), (200, 60), (2, 2), (1, 1), (100, 30)] {
		host.resize(UVec2::new(size.0, size.1));
		host.app.update();
		host.app.update();
	}
	host.resize(UVec2::new(120, 40));
	// the wheel above scrolled the page; jump back to the top so the heading is
	// in view, confirming the app is alive and still scrollable after the churn.
	host.send(b"\x1b[1~"); // Home
	host.step_until("Crates");
}

/// The app-wide [`ColorScheme`] resource (seeded by `--color-scheme=light` in
/// the binary) pins the scheme on every page, in place of the dark default.
#[beet::test]
async fn color_scheme_resource_pins_light() {
	let mut host = SiteHost::new_with(UVec2::new(120, 96), "/", |app| {
		app.insert_resource(AppColorScheme(ColorScheme::Light));
	});
	host.step_until("malleable application framework");
	host.has_class("light-scheme").xpect_true();
	host.has_class("dark-scheme").xpect_false();
	// the scheme survives navigation (it is session state, not a URL param)
	host.navigate("/docs/design/grid");
	host.step_until("12 columns");
	host.has_class("light-scheme").xpect_true();
}

/// The grid design page flows its numbered cells into column tracks: the
/// first demo's cells sit side by side on one track row.
#[beet::test]
async fn grid_page_flows_tracks() {
	let mut host = SiteHost::new(UVec2::new(120, 96), "/docs/design/grid");
	let frame = host.step_until("4 columns");
	// the 12-column demo's first cells share a row, advancing across tracks
	let (col_1, row_1) = host.cell_of("1");
	let (col_2, row_2) = host.cell_of("2");
	(col_2 > col_1).xpect_true();
	row_2.xpect_eq(row_1);
	// every cell of the 12-column demo rendered
	for index in 1..=12 {
		frame.as_str().xpect_contains(&index.to_string());
	}
}

/// The images page renders; without kitty graphics support (the host forces
/// it off, so nothing fetches) every demo shows its alt-text fallback, the
/// constrained boxes wrapping theirs.
#[beet::test]
async fn images_page_falls_back_to_alt_text() {
	let mut host = SiteHost::new(UVec2::new(120, 96), "/docs/design/images");
	let frame = host.step_until("[image]: teen titans, intrinsic size");
	frame
		.as_str()
		// the 20-cell box wraps its alt text, proving the sizing applies
		.xpect_contains("20rem wide")
		.xpect_contains("[image]: teen titans, stretched");
}

/// Scenario 2, the final boss: the form page is interactive in the terminal.
/// Tab and click focus the fields, typing a name then clicking Submit renders the
/// values as JSON below (the native counterpart of the web `<script>`).
#[beet::test]
async fn form_focus_type_and_submit() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/docs/design/form");
	host.step_until("Submit");

	// Click focusing: clicking the Name field focuses it.
	let (col, row) = host.element_cell("name");
	host.click(col, row);
	host.focused_name().xpect_eq(Some("name".to_string()));

	// Tab focusing moves through the form fields in order from there.
	host.send(b"\t");
	host.app.update();
	host.focused_name().xpect_eq(Some("email".to_string()));
	host.send(b"\t");
	host.app.update();
	host.focused_name().xpect_eq(Some("role".to_string()));

	// Re-focus Name, type a value.
	let (col, row) = host.element_cell("name");
	host.click(col, row);
	host.send(b"Ada Lovelace");
	for _ in 0..3 {
		host.app.update();
	}

	// Click Submit: the runtime writes the values as JSON into #form-output.
	let (col, row) = host.cell_of("Submit");
	host.click(col, row);
	host.step_until("\"name\": \"Ada Lovelace\"");
}
