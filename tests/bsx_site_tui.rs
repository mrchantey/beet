//! The live-TUI harness for the on-disk `bsx_site` example: the real `main.bsx`
//! entry loaded from disk through a [`BlobStore`] over an [`FsStore`], booted into
//! an in-process [`ChannelTerminal`] and navigated, asserting the rendered frame.
//!
//! The no-code twin of `examples/rsx_site/tests/tui.rs`: same `page_host` +
//! `Navigator` + channel-terminal harness, but the router is the example's markup
//! loaded from disk (the binary's `--server=tui` path) rather than a Rust
//! `rsx_site_router()`. Headless and deterministic, the behavioral gate the binary
//! cannot be in a sandbox.
beet::test_main!();

use beet::prelude::*;

#[path = "bsx_site/mod.rs"]
mod bsx_site;
use bsx_site::build_site;

/// A booted live-TUI site app: the on-disk router + a page host + an in-world
/// navigator, driven through a channel terminal.
struct SiteHost {
	app: App,
	host: Entity,
}

/// An SGR mouse sequence: button `button` at 0-indexed cell `(col, row)`, pressed
/// (`M`) or released (`m`).
fn sgr(button: u32, col: u32, row: u32, pressed: bool) -> Vec<u8> {
	let suffix = if pressed { 'M' } else { 'm' };
	format!("\x1b[<{button};{};{}{suffix}", col + 1, row + 1).into_bytes()
}

impl SiteHost {
	/// Boot the on-disk `examples/bsx_site` site at `home` with a `size`-cell
	/// viewport: build the markup entry into a router root (the same store-backed
	/// load `build_site_root` runs), settle the async `<RoutesDir>` scan, then pair
	/// a channel terminal with a page host and an in-world navigator on one surface,
	/// exactly as the binary's `--server=tui` boot composes them.
	async fn new(size: UVec2, home: &str) -> Self {
		let mut app = App::new();
		// the render substrate (`RouterPlugin` brings the BSX engine + server types
		// + `AsyncPlugin`) plus the live interactive plugins and the style rules, as
		// the binary's `tui` arm composes them.
		app.add_plugins((
			RouterPlugin,
			CharcellTuiPlugin,
			NavigatorPlugin,
			LivePagePlugin,
			material::MaterialStylePlugin,
		))
		.insert_resource(pkg_config!());
		// deterministic frames whatever terminal runs the tests: no kitty escapes.
		app.insert_resource(KittyGraphicsSupport { enabled: false });

		// the on-disk markup router, built into its own entity; the in-world
		// navigator dispatches to it.
		let router = build_site(app.world_mut()).await;
		// the host pairs a channel terminal with the page-host buffer, the in-world
		// navigator co-located on it (one surface), as the TUI boot composes them.
		let (channel, terminal) = ChannelTerminal::new(TerminalConfig::default());
		let host = app
			.world_mut()
			.spawn((
				channel,
				terminal,
				page_host(size),
				Navigator::in_world(router, home),
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
}

/// The homepage boots and renders, with the layout chrome and the dark scheme the
/// terminal target seeds.
#[beet::test]
async fn homepage_boots_with_chrome() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/").await;
	host.step_until("A site with no code");
	// the `<SiteLayout>` chrome renders: the header brand and the sidebar.
	let frame = host.frame();
	frame.as_str().xpect_contains("BSX Site");
	// the terminal target seeds the dark scheme (no web color-scheme script).
	host.has_class("dark-scheme").xpect_true();
}

/// Navigating to the no-code counter page renders its body, the in-world
/// navigation path the harness exists to cover.
#[beet::test]
async fn navigate_to_counter_page() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/").await;
	host.step_until("A site with no code");
	host.navigate("/counter");
	host.step_until("You have clicked 0 times.");
	host.frame().xnot().xpect_contains("A site with no code");
}

/// The no-code `routes/counter.bsx` is interactive in the terminal: clicking
/// "More" writes the `@doc:count` field, which document-sync fans to the display
/// binding, repainting the count. The markup twin of `rsx_site`'s Rust counter.
#[beet::test]
async fn counter_clicks_update_count() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/counter").await;
	host.step_until("You have clicked 0 times.");

	// click "More": the increment verb writes the field, the count repaints.
	let (col, row) = host.cell_of("More");
	host.click(col, row);
	host.step_until("You have clicked 1 times.");

	// click again: the same field accumulates.
	host.click(col, row);
	host.step_until("You have clicked 2 times.");

	// click "Less": the decrement verb walks the count back down.
	let (col, row) = host.cell_of("Less");
	host.click(col, row);
	host.step_until("You have clicked 1 times.");
}

/// Clicking the sidebar `counter` link (labelled by its path segment) navigates
/// the TUI to the counter page, the navigate-by-click path against the markup-built
/// `RouteSidebar`.
#[beet::test]
async fn nav_link_click_navigates() {
	let mut host = SiteHost::new(UVec2::new(120, 64), "/").await;
	host.step_until("A site with no code");
	// the sidebar grows a link per route, labelled by its last path segment
	// (`counter.bsx` -> "counter"); click it to navigate.
	let (col, row) = host.cell_of("counter");
	host.click(col, row);
	host.step_until("You have clicked 0 times.");
	host.frame().xnot().xpect_contains("A site with no code");
}
