//! Live reload round-tripped through a real browser: a [`PageHarness`] serves a
//! watched on-disk site shaped like `site/` (a `main.bsx` entry with a `<Theme>`,
//! a `templates/` dir holding the `SiteLayout` document and the `<Rule>` styles,
//! a `routes/` dir of markdown), the browser opens it, the test edits the files,
//! and the page's [`LiveReloadScript`] reloads into the edited content with no
//! navigation from the test.
//!
//! Two reload paths are covered. A *content* edit (a markdown route, the
//! per-request `Layout.bsx`) refreshes just that file's route or template in
//! place. A *structural* edit
//! (the entry's `<Theme>`, the `Styles.bsx` the entry instantiates once) tears
//! the entry scene down and rebuilds it through the exact `beet --watch` driver
//! path ([`entry_build::build_watched`]): the server rebinds and the browser
//! reconnects and reloads.
//!
//! Run: chromedriver + chromium on PATH, then
//! ```sh
//! cargo test --test live_reload_browser \
//!   --features router,markdown,fs,rand,client_io,template_serde,testing,webdriver -- --include-ignored
//! ```
beet::test_main!();

use beet::net::prelude::webdriver::*;
use beet::prelude::*;

/// The entry document, the site's `main.bsx` in miniature: the server owns the
/// boot, the router wraps every route in the `templates/Layout.bsx` document
/// and serves the default app routes (among them the `/__client_io` channel the
/// live-reload client joins), and the entry instantiates `<Styles/>` once at
/// build.
fn main_bsx(port: u16, primary: &str) -> String {
	format!(
		r#"<CallOnReady {{HttpServer{{port:{port}, canonical:false}}}}>
<Router {{Layout{{template:"Layout"}}}}>
	<PackageConfig title="Live"/>
	<Theme color="{primary}" primary="{primary}"/>
	<TemplateDir src="templates"/>
	<Styles/>
	<DefaultAppRoutes/>
	<RoutesDir src="routes"/>
</Router>
</CallOnReady>"#
	)
}

/// The document layout wrapping every route: the shipped chrome (which injects
/// the live-reload client) around the page body, with a marked footer.
fn layout_bsx(footer: &str) -> String {
	format!(r#"<SiteLayout><p slot="footer">{footer}</p></SiteLayout>"#)
}

/// The site's named rules: the `.probe` swatch's literal fill.
fn styles_bsx(probe_fill: &str) -> String {
	format!(r#"<Rule class="probe" background-color="{probe_fill}"/>"#)
}

/// The home page: a swatch the styles paint, and a line of body text.
fn index_md(body: &str) -> String {
	format!("# Home\n\n<div class=\"probe\">swatch</div>\n\n{body}")
}

/// A site fixture in a fresh system temp dir. Not under `target/`:
/// [`LiveReload`]'s default filter excludes that churn, so a fixture there
/// never reloads.
///
/// The entry declares a fixed port rather than `port=0`: a structural edit
/// rebuilds the scene and its server rebinds, and the browser's origin must
/// survive that to reconnect.
struct SiteFixture {
	dir: TempDir,
	port: u16,
}

impl SiteFixture {
	fn new() -> Self {
		let fixture = Self {
			dir: TempDir::new().unwrap(),
			port: HttpServer::free_port().unwrap(),
		};
		fixture.write("main.bsx", main_bsx(fixture.port, "#006c4f"));
		fixture.write("templates/Layout.bsx", layout_bsx("footer one"));
		fixture.write("templates/Styles.bsx", styles_bsx("#ff0000"));
		fixture.write("routes/index.md", index_md("first paint"));
		fixture
	}

	/// Write (or overwrite) a site file, the edit under test.
	fn write(&self, rel: &str, content: String) {
		fs_ext::write(self.dir.join(rel), content).unwrap();
	}

	/// Serve the fixture exactly as `beet --main=<dir> --watch` does: the
	/// entry resolves through its own fs repo store and builds through the
	/// `--watch` driver path, so its declared `<HttpServer>` boots on
	/// [`Self::port`] and a structural edit rebuilds the whole scene.
	async fn serve(&self) -> PageHarness {
		let dir = self.dir.to_string();
		let mut page = PageHarness::serve_app(move |app| {
			app.add_plugins((
				RouterPlugin,
				material::MaterialStylePlugin::default(),
			));
			let formats = app
				.world_mut()
				.get_resource_or_init::<TemplateFormats>()
				.clone();
			app.world_mut().run_async_local(async move |world| {
				let ResolvedEntry {
					repo_store,
					entry_name,
					..
				} = entry_build::resolve_main(None, None, &dir).await?;
				entry_build::build_watched(
					&world, repo_store, entry_name, formats,
				)
				.await
			});
		})
		.await
		.unwrap();
		// pin the scheme so the theme's tones resolve the same on every host
		page.goto("/?color-scheme=light").await.unwrap();
		page
	}
}

/// The computed background of the `.probe` swatch, eg `rgb(255, 0, 0)`. Errs
/// mid-reload, when there is no document to ask.
async fn probe_fill(page: &Page) -> Result<String> {
	page.evaluate_value(
		"getComputedStyle(document.querySelector('.probe')).backgroundColor",
	)
	.await?
	.as_str()
	.map(str::to_string)
	.ok_or_else(|| bevyhow!("fill is not a string"))
}

/// Poll `read` until it yields a value other than `previous`, returning it:
/// the reload lands asynchronously (the browser reloads itself), so a value is
/// read until it moves.
async fn until_changed(
	previous: &str,
	read: impl AsyncFn() -> Result<String>,
) -> String {
	poll_ext::poll_async_with(
		async || {
			// a read mid-reload (the document torn down) errors: keep polling
			let value = read().await?;
			(value != previous)
				.then_some(value)
				.ok_or_else(|| bevyhow!("still {previous}"))
		},
		Duration::from_secs(20),
		poll_ext::DEFAULT_INTERVAL,
	)
	.await
	.unwrap()
}

/// A markdown route edit takes the content path: the fs watcher's `BlobEvent`
/// respawns the routes, the reload broadcast reaches the browser over
/// `/__client_io`, and the page reloads itself into the edited body.
#[beet_core::test(timeout_ms = 120_000)]
#[ignore = "smoketest"]
async fn reloads_on_route_edit() {
	let site = SiteFixture::new();
	let page = site.serve().await;
	page.find_text("first paint").await;

	site.write("routes/index.md", index_md("second paint"));
	// the auto-waiting locator polls until the reloaded document paints it
	page.find_text("second paint").await;
	page.find("main")
		.await
		.xpect_not_contains_text("first paint")
		.await;

	page.kill().await.unwrap();
}

/// A per-request template edit (the `Layout.bsx` document) takes the content
/// path too: the template re-registers and the reloaded page renders through
/// the edited layout.
#[beet_core::test(timeout_ms = 120_000)]
#[ignore = "smoketest"]
async fn reloads_on_layout_edit() {
	let site = SiteFixture::new();
	let page = site.serve().await;
	page.find_text("footer one").await;

	site.write("templates/Layout.bsx", layout_bsx("footer two"));
	page.find_text("footer two").await;

	page.kill().await.unwrap();
}

/// A `<Rule>` colour edit in `Styles.bsx` reaches the browser: the entry
/// instantiates `<Styles/>` once at build, so the edit is structural (a full
/// teardown + rebuild), the server rebinds, the browser reconnects and reloads,
/// and the swatch paints the new fill with no stale copy of the old rule
/// winning the cascade.
#[beet_core::test(timeout_ms = 120_000)]
#[ignore = "smoketest"]
async fn reloads_on_style_rule_edit() {
	let site = SiteFixture::new();
	let page = site.serve().await;
	probe_fill(&page).await.unwrap().xpect_eq("rgb(255, 0, 0)");

	site.write("templates/Styles.bsx", styles_bsx("#0000ff"));
	until_changed("rgb(255, 0, 0)", async || probe_fill(&page).await)
		.await
		.xpect_eq("rgb(0, 0, 255)");
	// and again, so the second rebuild is as clean as the first
	site.write("templates/Styles.bsx", styles_bsx("#00ff00"));
	until_changed("rgb(0, 0, 255)", async || probe_fill(&page).await)
		.await
		.xpect_eq("rgb(0, 255, 0)");

	page.kill().await.unwrap();
}

/// A `<Theme>` edit in the entry document is structural: the rebuilt scene
/// re-derives the palette tones and the reloaded page's root tone variables
/// agree with the freshly served stylesheet.
#[beet_core::test(timeout_ms = 120_000)]
#[ignore = "smoketest"]
async fn reloads_on_theme_edit() {
	let site = SiteFixture::new();
	let page = site.serve().await;
	let tone = async || {
		page.evaluate_value(
			"getComputedStyle(document.documentElement).getPropertyValue('--material-tones-primary40').trim()",
		)
		.await?
		.as_str()
		.map(str::to_string)
		.ok_or_else(|| bevyhow!("tone is not a string"))
	};
	let before = tone().await.unwrap();
	before.is_empty().xpect_false();

	site.write("main.bsx", main_bsx(site.port, "#1a4fff"));
	let after = until_changed(&before, tone).await;
	// the browser's tone is the one the rebuilt server now bakes
	Request::get(page.route_url("/?color-scheme=light"))
		.send()
		.await
		.unwrap()
		.text()
		.await
		.unwrap()
		.xpect_contains(&format!("--material-tones-primary40: {after};"));

	page.kill().await.unwrap();
}
