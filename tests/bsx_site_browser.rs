//! The browser half of the no-code `examples/bsx_site` example: the served
//! page boots the wasm `beet` binary on the same entry it was rendered by, and
//! the counter counts again through the world running in the tab.
//!
//! The site is served exactly as `beet --main=examples/bsx_site` serves it
//! (the entry resolved through its own fs repo store, built through the plain
//! one-shot path, its declared `<HttpServer>` booting), the harness reading the
//! url off the listener the entry binds. The page's `<Wasm/>` boots the entry
//! again in the browser under `--server=dom`; once its `DomHost` paints, a
//! trusted click on `More` reaches the button's entity, its `bx:click` script
//! runs in the iframe backend, and the bound count repaints in place.
//!
//! Needs the artifact and a browser on PATH:
//!
//! ```sh
//! just test-bsx-site-browser   # builds beet-ui.wasm, then runs this
//! ```
beet::test_main!();

use beet::net::prelude::webdriver::*;
use beet::prelude::*;

/// What the `DomHost` logs once it has painted the counter page.
const PAINTED: &str = "dom host painted /counter";

/// Serve the on-disk example the way the binary does, on an OS-assigned port.
async fn serve_site() -> PageHarness {
	PageHarness::serve_app(|app| {
		app.add_plugins((
			RouterPlugin,
			material::MaterialStylePlugin::default(),
		));
		let formats = app
			.world_mut()
			.get_resource_or_init::<TemplateFormats>()
			.clone();
		app.world_mut().run_async_local(async move |world| {
			let dir = AbsPath::new_workspace_rel("examples/bsx_site")?;
			let ResolvedEntry {
				repo_store,
				entry_name,
				prescan,
				..
			} = entry_build::resolve_main(None, None, dir.as_str()).await?;
			let sources = entry_build::read_sources(
				&repo_store,
				formats,
				entry_name,
				prescan,
			)
			.await?;
			world
				.with(move |world: &mut World| {
					entry_build::build_root(
						world, repo_store, sources, RepoStore,
					)
					.map(|_| ())
				})
				.await
		});
	})
	.await
	.unwrap()
}

/// Drain the console into `log` until `needle` appears, failing on a panic or
/// the deadline.
async fn console_until(
	console: &Collector<ConsoleEntry>,
	log: &mut String,
	needle: &str,
) {
	let deadline = Instant::now() + Duration::from_secs(120);
	while !log.contains(needle) {
		for entry in console.drain() {
			cross_log!("{}", entry.text);
			log.push_str(&entry.text);
			log.push('\n');
		}
		if log.contains("panicked") {
			panic!("the browser process panicked. console:\n{log}");
		}
		if Instant::now() > deadline {
			panic!("the console never said `{needle}`. console:\n{log}");
		}
		time_ext::sleep(Duration::from_millis(250)).await;
	}
}

#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn the_counter_counts_in_the_browser() {
	let artifact =
		AbsPath::new_workspace_rel("assets/wasm/beet-ui.wasm").unwrap();
	if !fs_ext::exists(&artifact).unwrap_or_default() {
		panic!("missing artifact `{artifact}`, run `just build-wasm-ui`");
	}
	// the launch, read once the first server spawns: only the http server boots
	// (the cli render would exit the app), on an OS port rather than the default
	// so the suite never fights a running dev server
	unsafe {
		env_ext::set_var("BEET_SERVER", "http").unwrap();
		env_ext::set_var("BEET_HTTP_PORT", "0").unwrap();
	}
	let mut page = serve_site().await;
	let console = page.console().await.unwrap();
	let responses = page.responses().await.unwrap();
	page.goto("/counter").await.unwrap();
	// the first paint is the served page
	page.find_text("You have clicked 0 times.").await;
	// the boot: the entry resolves through `/repo`, its `DomServer` lands the
	// navigator on this page and paints it
	let mut log = String::new();
	console_until(&console, &mut log, PAINTED).await;
	// the loop: a trusted click reaches the world, the script runs, the count
	// repaints in place
	page.click_text("More").await.unwrap();
	page.find_text("You have clicked 1 times.").await;
	page.click_text("More").await.unwrap();
	page.find_text("You have clicked 2 times.").await;
	page.click_text("Less").await.unwrap();
	page.find_text("You have clicked 1 times.").await;
	for entry in console.drain() {
		log.push_str(&entry.text);
		log.push('\n');
	}
	log.as_str().xnot().xpect_contains("ERROR");
	responses
		.drain()
		.into_iter()
		.filter(|response| response.is_error())
		.map(|response| format!("{} {}", response.status, response.url))
		.collect::<Vec<_>>()
		.xpect_empty();
	page.kill().await.unwrap();
}
