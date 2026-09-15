//! The committed page-driving checks for the browser bootstrap: a page whose
//! `<Wasm repo main [server]>` names an entry boots the built binary in
//! headless chromium through the in-house webdriver, and the console says the
//! entry resolved through the http repo and ran. The whole launch path the
//! terminal and the server share, run in a tab:
//!
//! - the headless boot: `beet-min.wasm` on `examples/wasm/hello.bsx`, the
//!   `just serve-wasm` page's launch, logs the greeting and exits;
//! - the DOM boot: `beet-ui.wasm` on `examples/ui/scene_editor.bsx` with
//!   `--server=dom` boots its `DomServer`, lands its navigator on `/`, the
//!   page's own url, paints the scene into the body once it settles, and
//!   takes input: a click on a tree row selects it and the inspector appears.
//!
//! Ignored by default since they need the artifacts and a browser on PATH:
//!
//! ```sh
//! just build-wasm-min build-wasm-ui   # the artifacts under test
//! just check-wasm-boot                # these checks (needs chromedriver + a chromium)
//! ```

use super::browser_check::*;
use beet::prelude::webdriver::*;
use beet::prelude::*;

/// Serve `page`, open it, and drain the console until `needle` appears,
/// failing on a panic or the deadline; the log so far and the still-open
/// browser are returned for further assertions.
async fn boot_until(page: String, needle: &str) -> (String, Browser) {
	let port = serve_wasm_page(page).await.unwrap();
	let url = format!("http://127.0.0.1:{port}/");
	let mut browser = Browser::new_with_opts(driver().unwrap(), default())
		.await
		.unwrap();
	let console = browser.console().await.unwrap();
	browser.navigate(&url).await.unwrap();
	// the module fetch, the entry's reads over http, the boot
	let mut log = String::new();
	let deadline = Instant::now() + Duration::from_secs(120);
	while !log.contains(needle) {
		drain(&console, &mut log);
		if log.contains("panicked") {
			panic!("the browser process panicked. console:\n{log}");
		}
		if Instant::now() > deadline {
			panic!("the console never said `{needle}`. console:\n{log}");
		}
		time_ext::sleep(Duration::from_millis(500)).await;
	}
	// whatever follows the landing lands too
	time_ext::sleep(Duration::from_secs(2)).await;
	drain(&console, &mut log);
	(log, browser)
}

#[beet::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-min` + chromedriver"]
async fn browser_headless_boot() {
	require_artifact("assets/wasm/beet-min.wasm", "just build-wasm-min");
	let page = wasm_page(rsx! {
		<Wasm src="/assets/wasm/beet-min.wasm" repo="/examples/wasm" main="hello.bsx"/>
	})
	.unwrap();
	let (log, browser) = boot_until(
		page,
		"Edit examples/wasm/hello.bsx and the page live-reloads.",
	)
	.await;
	browser.kill().await.unwrap();
	log.as_str()
		.xpect_contains("Hello from beet, running headless in your browser.")
		.xnot()
		.xpect_contains("ERROR");
}

#[beet::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn browser_dom_boot() {
	require_artifact("assets/wasm/beet-ui.wasm", "just build-wasm-ui");
	let page = wasm_page(rsx! {
		<Wasm src="/assets/wasm/beet-ui.wasm" repo="/examples/ui" main="scene_editor.bsx" server="dom"/>
	})
	.unwrap();
	// what the DOM host logs once its navigator binds the page at the request
	// path and, the scene having landed, paints it (see `DomHost`); the run
	// then parks on the server, still up and quiet
	let (log, browser) = boot_until(page, "dom host painted /").await;
	// the world's paint replaced the loader page: the scene's heading is in
	// the document, bound to its entity
	let heading = browser
		.evaluate_value(
			"[document.querySelector('h1')?.textContent, \
			 typeof document.querySelector('h1')?.beetEntity].join(':')",
		)
		.await
		.unwrap()
		.to_string();
	// the input path: open the editor (the disclosure is the browser's), then
	// a trusted click on a tree row reaches its entity, selects it, and the
	// inspector generates for it
	browser.click("summary").await.unwrap();
	browser.click(".scene-tree-row").await.unwrap();
	browser.find(".scene-inspector-title").await;
	browser
		.find(".scene-tree-row")
		.await
		.xpect_attr("aria-selected", "true")
		.await;
	browser.kill().await.unwrap();
	log.as_str()
		.xpect_contains("dom host landed /")
		.xnot()
		.xpect_contains("ERROR");
	heading.xpect_contains("Garden:bigint");
}
