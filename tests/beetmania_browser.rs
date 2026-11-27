//! The browser half of the no-code `examples/beetmania` example: the served
//! page boots the wasm `beet` binary on the same entry it was rendered by, the
//! world adopts every route's served page as it stands, and the counter counts
//! again through the world running in the tab.
//!
//! The site is served exactly as `beet --entry=examples/beetmania` serves it
//! (the entry resolved through its own fs package store, built through the plain
//! one-shot path, its declared `<HttpServer>` booting), the harness reading the
//! url off the listener the entry binds. The page's `<Wasm/>` boots the entry
//! again in the browser under `--server=dom`; once its `DomHost` paints, a
//! trusted click on `Ate one` reaches the button's entity, its `bx:click` script
//! runs in the iframe backend, and the bound count repaints in place.
//!
//! Needs the wasm runtime and a browser on PATH:
//!
//! ```sh
//! just test-beetmania-browser   # builds beet-ui.wasm, then runs this
//! ```
beet::test_main!();

mod browser_host;
use browser_host::*;
#[path = "beetmania/mod.rs"]
mod beetmania;
use beetmania::build_site;

use beet::prelude::*;

/// The example's directory, the entry's source directory.
const ENTRY_DIR: &str = "examples/beetmania";

/// Serve the on-disk example the way the binary does, on an OS-assigned port.
async fn serve_site() -> BrowserHost {
	BrowserHost::serve(BrowserHost::disk_store(ENTRY_DIR), "main.bsx", "serve")
		.await
}

/// Every page the example's router serves, by url path: read off its route
/// tree, so a post's `slug` (rather than its numbered filename) is the url.
async fn routes() -> Vec<String> {
	let mut world =
		(AsyncPlugin, RouterPlugin, material::MaterialStylePlugin).into_world();
	let root = build_site(&mut world).await;
	let router = world
		.run_system_cached_with::<_, Result<Entity>, _, _>(find_router, root)
		.unwrap()
		.unwrap();
	let mut routes = RouteTree::of(&world, router)
		.unwrap()
		.flatten_nodes()
		.into_iter()
		.filter(|node| node.is_page_route)
		.map(|node| {
			format!("/{}", node.path.to_string().trim_start_matches('/'))
		})
		.collect::<Vec<_>>();
	routes.sort();
	routes
}

#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn the_counter_counts_in_the_browser() {
	let mut host = serve_site().await;
	host.goto("/counter").await;
	// the first paint is the served page
	host.find_text("Beets eaten today: 0").await;
	// the boot: the entry resolves through `/package`, its `DomServer` lands the
	// navigator on this page and adopts it as it stands
	host.painted_clean("/counter").await;
	// the loop: a trusted click reaches the world, the script runs, the count
	// repaints in place
	host.click_text("Ate one").await.unwrap();
	host.find_text("Beets eaten today: 1").await;
	host.click_text("Ate one").await.unwrap();
	host.find_text("Beets eaten today: 2").await;
	host.click_text("Put one back").await.unwrap();
	host.find_text("Beets eaten today: 1").await;
	host.kill().await;
}

/// A pinned scheme reaches the browser chrome: whatever the OS prefers, the
/// pinned scheme's `theme-color` matches and the other never does, through
/// the boot.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn a_pinned_scheme_tints_the_browser() {
	let mut host = serve_site().await;
	host.goto("/counter?color-scheme=dark").await;
	let tinted = async |host: &BrowserHost| {
		host.find(r#"meta[name="theme-color"][data-scheme="dark"]"#)
			.await
			.xpect_attr("media", "all")
			.await;
		host.find(r#"meta[name="theme-color"][data-scheme="light"]"#)
			.await
			.xpect_attr("media", "not all")
			.await;
	};
	tinted(&host).await;
	// the world opens at the page's own url, its query included
	host.painted("/counter?color-scheme=dark").await;
	tinted(&host).await;
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
