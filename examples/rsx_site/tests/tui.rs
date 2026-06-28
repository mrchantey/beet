//! The live-TUI harness: navigation and native reactivity driven against
//! `rsx_site`'s real router through an in-process `ChannelTerminal`.
//!
//! Mirrors `main.rs`'s `tui` wiring (the live plugins + `rsx_site_router`) but
//! swaps the `StdioTerminal` for a `ChannelTerminal`, so it is headless and
//! deterministic — the behavioral gate the binary cannot be in a sandbox.
beet::test_main!();

use beet::prelude::*;
use rsx_site::prelude::*;

/// A booted live-TUI site app: the real router + a page host + an in-world
/// navigator, driven through a channel terminal.
struct SiteHost {
	app: App,
	host: Entity,
}

/// An SGR mouse sequence: button `b` at 0-indexed cell `(col, row)`, pressed
/// (`M`) or released (`m`).
fn sgr(button: u32, col: u32, row: u32, pressed: bool) -> Vec<u8> {
	let suffix = if pressed { 'M' } else { 'm' };
	format!("\x1b[<{button};{};{}{suffix}", col + 1, row + 1).into_bytes()
}

impl SiteHost {
	/// Boot the site at `home` with a `size`-cell viewport.
	fn new(size: UVec2, home: &str) -> Self {
		Self::new_with(size, home, |_| {})
	}

	/// Boot the site at `home`, running `setup` before the first frame — for app
	/// state that must exist when the initial page renders (eg the session
	/// [`ColorScheme`], which the binary seeds before navigating).
	fn new_with(size: UVec2, home: &str, setup: impl FnOnce(&mut App)) -> Self {
		let mut app = App::new();
		// the shared site substrate (router + style rules + package config) plus the
		// live interactive plugins, exactly as `main.rs`'s `tui` arm composes them.
		app.add_plugins(server_plugin)
			.insert_resource(PackageConfig {
				title: "Beet".into(),
				..pkg_config!()
			})
			.add_plugins((CharcellTuiPlugin, NavigatorPlugin, LivePagePlugin));
		setup(&mut app);

		// the router on its own entity; the in-world navigator dispatches to it.
		let router = app.world_mut().spawn(rsx_site_router()).id();
		// the host pairs a channel terminal with the page-host buffer, the in-world
		// navigator co-located on it (one surface), as the TUI boot composes them.
		let (channel, terminal) =
			ChannelTerminal::new(TerminalConfig::default());
		let host = app
			.world_mut()
			.spawn((
				channel,
				terminal,
				page_host(size),
				Navigator::in_world(router, home),
				// deterministic frames whatever terminal runs the tests: graphics off
				// per surface, so no kitty escapes interleave with the painted cells.
				KittyGraphicsSupport { enabled: false },
			))
			.id();
		app.update();
		Self { app, host }
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
			.entity_mut(self.host)
			.run_async_local(move |entity| Navigator::navigate_to(entity, url));
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
	///
	/// Fullwidth glyphs (large-font text) are folded back to ASCII before the
	/// match, so a plain-ASCII needle finds text painted at either width.
	fn step_until(&mut self, needle: &str) -> String {
		for _ in 0..200 {
			self.app.update();
			let frame = self.frame();
			if from_fullwidth(&frame).contains(needle) {
				return frame;
			}
		}
		panic!("frame never contained '{needle}':\n{}", self.frame());
	}
}

/// The homepage boots and renders, with the layout chrome and the dark scheme
/// the terminal target seeds.
#[beet::test]
async fn homepage_boots_with_chrome_and_scheme() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/");
	host.step_until("malleable application framework");
	// the BaseLayout chrome renders: header nav and footer.
	let frame = host.frame();
	frame
		.as_str()
		.xpect_contains("Counter")
		.xpect_contains("Buttons");
	frame.xpect_contains("© Beet");
	// the terminal target seeds the dark scheme (no web color-scheme script).
	host.has_class("dark-scheme").xpect_true();
}

/// The terminal app fills at least the full viewport height, like the web, so a
/// short page pins its footer to the bottom rows rather than sitting
/// content-height with the footer floating just below the content.
#[beet::test]
async fn app_fills_terminal_height() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/counter");
	host.step_until("You have clicked 0 times.");
	let frame = host.frame();
	let footer_row = frame
		.lines()
		.position(|line| line.contains("© Beet"))
		.expect("footer present");
	// the short counter content is ~15 rows; the footer is pushed to the bottom of
	// the 64-row viewport by the viewport-fill page column, not floating near it.
	(footer_row >= 60).xpect_true();
}

/// Clicking the header `Counter` nav link navigates the TUI to the counter page,
/// the navigate-by-click path the harness exists to cover.
#[beet::test]
async fn nav_link_click_navigates() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/");
	host.step_until("malleable application framework");
	// click the header "Counter" link -> navigate to the counter page.
	let (col, row) = host.cell_of("Counter");
	host.click(col, row);
	host.step_until("You have clicked 0 times.");
	from_fullwidth(&host.frame())
		.xnot()
		.xpect_contains("malleable application framework");
}

/// The Rust reactive counter is interactive in the terminal: clicking a button
/// mutates the document field, which document-sync fans to the display binding,
/// repainting the count. The Rust mirror of the no-code `counter.bsx` page.
#[beet::test]
async fn counter_clicks_update_count() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/counter");
	host.step_until("You have clicked 0 times.");

	// click "More": the increment observer writes the field, the count repaints.
	let (col, row) = host.cell_of("More");
	host.click(col, row);
	host.step_until("You have clicked 1 times.");

	// click again: the same field accumulates.
	host.click(col, row);
	host.step_until("You have clicked 2 times.");

	// click "Less": the decrement observer walks the count back down.
	let (col, row) = host.cell_of("Less");
	host.click(col, row);
	host.step_until("You have clicked 1 times.");
}

/// The app-wide [`Theme::scheme`] (seeded by `--color-scheme=light` in the
/// binary) pins the scheme on every page, in place of the dark default, and
/// survives navigation (it is session state, not a URL param).
#[beet::test]
async fn color_scheme_resource_pins_light() {
	let mut host = SiteHost::new_with(UVec2::new(120, 64), "/", |app| {
		app.world_mut().get_resource_or_init::<Theme>().scheme =
			ColorScheme::Light;
	});
	host.step_until("malleable application framework");
	host.has_class("light-scheme").xpect_true();
	host.has_class("dark-scheme").xpect_false();
	host.navigate("/buttons");
	host.step_until("Outlined");
	host.has_class("light-scheme").xpect_true();
}
