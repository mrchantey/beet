//! The browser half of the no-code `examples/bsx_site` example: the served
//! page boots the wasm `beet` binary on the same entry it was rendered by, the
//! world adopts every route's served page as it stands, and the counter counts
//! again through the world running in the tab.
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

mod browser_host;
use browser_host::*;

use beet::prelude::*;

/// The example's directory, the entry's repo root.
const ENTRY_DIR: &str = "examples/bsx_site";

/// Serve the on-disk example the way the binary does, on an OS-assigned port.
async fn serve_site() -> BrowserHost {
	BrowserHost::serve(BrowserHost::disk_store(ENTRY_DIR), "main.bsx").await
}

/// Every route under the example's `routes/` directory, by url path.
async fn routes() -> Vec<String> {
	let dir = AbsPath::new_workspace_rel(ENTRY_DIR)
		.unwrap()
		.join("routes");
	let mut routes: Vec<String> = BlobStore::new(FsStore::new(dir))
		.list()
		.await
		.unwrap()
		.into_iter()
		.map(|path| {
			let stem = path.with_extension("").to_string();
			let stem = stem.strip_suffix("index").unwrap_or(&stem);
			format!("/{}", stem.trim_end_matches('/'))
		})
		.collect();
	routes.sort();
	routes
}

#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn the_counter_counts_in_the_browser() {
	let mut host = serve_site().await;
	host.goto("/counter").await;
	// the first paint is the served page
	host.find_text("You have clicked 0 times.").await;
	// the boot: the entry resolves through `/repo`, its `DomServer` lands the
	// navigator on this page and adopts it as it stands
	host.painted_clean("/counter").await;
	// the loop: a trusted click reaches the world, the script runs, the count
	// repaints in place
	host.click_text("More").await.unwrap();
	host.find_text("You have clicked 1 times.").await;
	host.click_text("More").await.unwrap();
	host.find_text("You have clicked 2 times.").await;
	host.click_text("Less").await.unwrap();
	host.find_text("You have clicked 1 times.").await;
	host.kill().await;
}

/// The conformance probe over the whole site: every route's served page is
/// adopted by the world that boots on it with nothing replaced or patched,
/// and the served chrome keeps working through the boot (the sidebar's menu
/// button binds its own script to the served node, which adoption keeps).
#[beet_core::test(timeout_ms = 600_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn every_route_adopts_untouched() {
	let mut host = serve_site().await;
	let routes = routes().await;
	routes.len().xpect_greater_than(3);
	for route in &routes {
		host.goto(route).await;
		host.painted_clean(route).await;
	}
	// the served menu button's own script survives the boot: a click still
	// flips the rail
	let rail = host.find("#sidebar").await;
	let hidden = rail.get_attribute("aria-hidden").await.unwrap();
	host.click("#menu-button").await.unwrap();
	let flipped = match hidden.as_deref() {
		Some("true") => "false",
		_ => "true",
	};
	rail.xpect_attr("aria-hidden", flipped).await;
	host.kill().await;
}
