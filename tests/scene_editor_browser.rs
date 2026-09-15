//! The browser half of the scene editor example: the served page boots the
//! wasm `beet` binary on the same entry it was rendered by, and the boot is
//! invisible.
//!
//! The page is a launch description whose wasm loads on intent (opening the
//! editor's disclosure), so a visitor who only reads it loads nothing. When
//! the world arrives it adopts the served page node for node rather than
//! repainting it, replays what was clicked while it loaded, and a returning
//! editor, whose fork the served page cannot show, never sees the published
//! text: their page is hidden until the fork paints.
//!
//! The example is served the way `beet --main=examples/ui/scene_editor.bsx`
//! serves it, from an in-memory copy of the entry, its scene, its shell and
//! the artifact, so no on-disk fork leaks in. Needs the artifact and a
//! browser on PATH:
//!
//! ```sh
//! just test-scene-editor-browser   # builds beet-ui.wasm, then runs this
//! ```
beet::test_main!();

mod browser_host;
use browser_host::*;

use beet::net::prelude::webdriver::*;
use beet::prelude::*;

/// The example's directory, the entry's repo root.
const ENTRY_DIR: &str = "examples/ui";
/// The entry document within it.
const ENTRY: &str = "scene_editor.bsx";
/// What the entry's store must hold: the entry, the authored scene, the
/// document shell, and the artifact its page boots (through the `assets`
/// link into the workspace tree).
const SEEDED: &[&str] = &[
	ENTRY,
	"scene_editor/scene.bsx",
	"templates/EditorShell.bsx",
	"assets/wasm/beet-ui.wasm",
	"assets/wasm/beet-ui.js",
];
/// The local-storage bit the fork sets, as the page names its repo.
const FORK_MARK: &str = "beet:fork:http:repo";

/// Serve the example, the tab open on nothing yet.
async fn serve_example() -> PageHarness {
	require_artifact();
	let store = seeded_store(ENTRY_DIR, SEEDED).await;
	serve_entry(store, ENTRY).await
}

/// Click the tree row whose label contains `label`.
async fn click_row(page: &Page, label: &str) {
	for row in page.find_all(".scene-tree-row").await {
		if row
			.text_content()
			.await
			.unwrap()
			.is_some_and(|text| text.contains(label))
		{
			row.click().await.unwrap();
			return;
		}
	}
	panic!("no tree row labelled `{label}`");
}

/// A click before the wasm has booted lands after it: opening the editor is
/// the intent that loads the wasm, a row clicked while it loads is queued by
/// the pre-boot script, and once the world has adopted the served page (with
/// nothing replaced or patched) the replay selects the row and the inspector
/// generates for it.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn a_pre_boot_click_lands_after_boot() {
	let mut page = serve_example().await;
	let console = page.console().await.unwrap();
	let responses = page.responses().await.unwrap();
	page.goto("/").await.unwrap();
	page.find_text("Garden").await;
	// the served page is inert: the disclosure is the browser's, and its
	// toggle is what injects the loader
	page.click("summary").await.unwrap();
	click_row(&page, "#2 \"Garden\"").await;
	// both clicks are queued: the wasm is still on its way
	page.evaluate_value(&format!(
		"globalThis.{}?.q.length ?? -1",
		PreBoot::GLOBAL
	))
	.await
	.unwrap()
	.as_i64()
	.xpect_eq(Some(2));
	let mut log = String::new();
	console_until(&console, &mut log, "dom host painted /").await;
	assert_adopted_clean(&log, "/");
	// the replay: the row is selected and the inspector generated for it
	page.find(".scene-inspector-title")
		.await
		.xpect_contains_text("#2 \"Garden\"")
		.await;
	page.find(".scene-tree-row[aria-selected=\"true\"]")
		.await
		.xpect_contains_text("#2 \"Garden\"")
		.await;
	drain(&console, &mut log);
	assert_no_errors(&log, &responses);
	page.kill().await.unwrap();
}

/// Booting is invisible: the served page and the page the world paints are
/// the same document, pixel for pixel and node for node.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn booting_is_invisible() {
	let mut page = serve_example().await;
	let console = page.console().await.unwrap();
	let responses = page.responses().await.unwrap();
	page.goto("/").await.unwrap();
	page.find_text("Garden").await;
	let before = page.screenshot().await.unwrap();
	let html_before = page
		.evaluate_value("document.body.innerHTML")
		.await
		.unwrap();
	// the intent without the gesture, so the page is exactly as served when
	// the world adopts it
	page.evaluate(
		"document.querySelector('details').dispatchEvent(new Event('toggle'))",
	)
	.await
	.unwrap();
	let mut log = String::new();
	console_until(&console, &mut log, "dom host painted /").await;
	assert_adopted_clean(&log, "/");
	let after = page.screenshot().await.unwrap();
	let html_after = page
		.evaluate_value("document.body.innerHTML")
		.await
		.unwrap();
	html_after.xpect_eq(html_before);
	(after == before).xpect_true();
	drain(&console, &mut log);
	assert_no_errors(&log, &responses);
	page.kill().await.unwrap();
}

/// A returning editor never sees the published text: an edit forks the page
/// into the browser's store and marks the browser, and the next load hides
/// the served body and boots at once, revealing the page only once the fork
/// has painted over it.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn a_returning_editor_never_sees_the_published_text() {
	let mut page = serve_example().await;
	let console = page.console().await.unwrap();
	let responses = page.responses().await.unwrap();
	page.goto("/").await.unwrap();
	page.find_text("Garden").await;
	page.click("summary").await.unwrap();
	let mut log = String::new();
	console_until(&console, &mut log, "dom host painted /").await;
	// the edit: the heading's text, retyped through its inspector
	click_row(&page, "#2 \"Garden\"").await;
	page.find(".scene-inspector-title").await;
	page.find(".scene-inspector input[type=\"text\"]")
		.await
		.type_text("!")
		.await
		.unwrap();
	page.find_text("Garden!").await;
	// the fork lands in the browser's store, marking it as an editor's
	poll_ext::poll_async(async || {
		match page
			.evaluate_value(&format!("localStorage.getItem({FORK_MARK:?})"))
			.await?
			.as_str()
		{
			Some("1") => Ok(()),
			other => bevybail!("the fork is not marked yet: {other:?}"),
		}
	})
	.await
	.unwrap();
	// the next load: hidden, and booting without a gesture
	drain(&console, &mut log);
	log.clear();
	page.goto("/").await.unwrap();
	let mut samples = 0;
	console_until_with(&console, &mut log, "dom host painted /", async || {
		let sample = page
			.evaluate_value(
				"[getComputedStyle(document.body).visibility, document.querySelector('h1').textContent]",
			)
			.await
			.unwrap();
		let (visibility, heading) = (
			sample[0].as_str().unwrap().to_string(),
			sample[1].as_str().unwrap().to_string(),
		);
		samples += 1;
		(visibility == "hidden" || heading == "Garden!")
			.xpect_true();
	})
	.await;
	samples.xpect_greater_than(0);
	// revealed, and the fork is what shows
	page.find_text("Garden!").await;
	page.evaluate_value("getComputedStyle(document.body).visibility")
		.await
		.unwrap()
		.as_str()
		.xpect_eq(Some("visible"));
	page.xpect_no_selector(&format!("#{}", PreBoot::STYLE_ID))
		.await;
	drain(&console, &mut log);
	assert_no_errors(&log, &responses);
	page.kill().await.unwrap();
}
