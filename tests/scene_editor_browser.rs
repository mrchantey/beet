//! The browser acceptance suite for the on-disk `examples/ui/scene_editor.bsx`
//! page, the twin of `scene_editor_tui.rs`: the served page boots the wasm
//! `beet` binary on the same entry it was rendered by, and the editor edits
//! the page in the tab.
//!
//! This is the scene editor's embodiment gate on the web. Every widget is
//! pinned in its own harness and the DOM sink and input path under
//! `#[beet_core::test(browser)]`; what only this suite can show is that an
//! *authored* entry assembles them in a real browser: the server forks the
//! page and publishes it, the tab boots the entry through the published
//! store, every control edits the document with the live page following, the
//! first edit forks the page into the browser's own store, and a reload
//! reproduces the edited page from it.
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
//! the artifact (the shared [`BrowserHost`]), so no on-disk fork leaks in.
//! Needs the artifact and a browser on PATH:
//!
//! ```sh
//! just test-scene-editor-browser   # builds beet-ui.wasm, then runs this
//! ```
beet::test_main!();

mod browser_host;
use browser_host::*;

use beet::net::prelude::webdriver::*;
use beet::prelude::*;
use std::ops::Deref;
use std::ops::DerefMut;

/// The example's directory, the entry's repo root.
const ENTRY_DIR: &str = "examples/ui";
/// The entry document within it.
const ENTRY: &str = "scene_editor.bsx";
/// The authored original of the scene, as the entry names it.
const SCENE: &str = "scene_editor/scene.bsx";
/// The fork, as the entry names it: the server writes it into its store on
/// its first boot and publishes it, the browser writes it into its own store
/// on the first edit.
const FORK: &str = "scene_editor/scene.json";
/// What the entry's store must hold: the entry, the authored scene, the
/// document shell, and the artifact its page boots (through the `assets`
/// link into the workspace tree).
const SEEDED: &[&str] = &[
	ENTRY,
	SCENE,
	"templates/EditorShell.bsx",
	"assets/wasm/beet-ui.wasm",
	"assets/wasm/beet-ui.js",
];
/// The local-storage bit the fork sets, as the page names its repo.
const FORK_MARK: &str = "beet:fork:http:repo";
/// The browser's fork of the repo, as the launch names it
/// (`indexed-db://beet/repo`): the database, its one object store, and the
/// fork's key within it.
const FORK_DB: &str = "beet";
const FORK_OBJECT_STORE: &str = "blobs";
const FORK_KEY: &str = "repo/scene_editor/scene.json";

/// The served page in a driven tab: the shared [`BrowserHost`] over the scene
/// editor entry, plus the scene reads and the editor gestures its cases make.
struct SceneHost(BrowserHost);

impl Deref for SceneHost {
	type Target = BrowserHost;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for SceneHost {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl SceneHost {
	/// Serve the example over a store holding the entry, the authored
	/// original, the shell and the artifact alone, so the server's boot is a
	/// first boot, and open the page: served, inert, the wasm not yet asked
	/// for.
	async fn new() -> Self {
		let store = BrowserHost::seeded_store(ENTRY_DIR, SEEDED).await;
		let mut host = Self(BrowserHost::serve(store, ENTRY).await);
		host.goto("/").await;
		host.find_text("Garden").await;
		host
	}

	/// Load the page again: the next session. A browser holding a fork boots
	/// at once behind a hidden body; wait for the paint that reveals it.
	async fn reload(&mut self) {
		self.goto("/").await;
		self.painted("/").await;
	}

	/// Open the editor's disclosure, the intent that loads the wasm on a
	/// first visit, and wait for the world to paint and its tree to show.
	async fn open_editor(&mut self) {
		self.click("summary").await.unwrap();
		self.painted("/").await;
		self.find_row("#14 ToggleSceneEditor").await;
	}

	/// Every tree row's text in display order, its guides folded to spaces
	/// so a needle reads as the terminal paints it.
	async fn rows(&self) -> Vec<String> {
		let mut rows = Vec::new();
		for row in self.find_all(".scene-tree-row").await {
			rows.push(fold(
				row.text_content().await.unwrap().unwrap_or_default(),
			));
		}
		rows
	}

	/// The first element matching `selector` whose text contains `needle`,
	/// polled fresh each time since a generation replaces its rows: a
	/// reference into the tree or the inspector's title goes stale under an
	/// edit, so a wait names the text rather than holding a node.
	async fn find_containing(
		&self,
		selector: &str,
		needle: &str,
	) -> WebElement {
		let page = &self.page;
		poll_ext::poll_async(async || {
			for element in page.try_find_all(selector).await? {
				let text = element.text_content().await?.unwrap_or_default();
				if fold(text).contains(needle) {
					return Ok(element);
				}
			}
			bevybail!("no `{selector}` containing {needle:?}")
		})
		.await
		.unwrap_or_else(|err| panic!("{err}"))
	}

	/// The tree row whose label contains `label`.
	async fn find_row(&self, label: &str) -> WebElement {
		self.find_containing(".scene-tree-row", label).await
	}

	/// The guides the row labelled `label` draws its place with.
	async fn guides(&self, label: &str) -> String {
		self.find_row(label)
			.await
			.find(".scene-tree-guides")
			.await
			.text_content()
			.await
			.unwrap()
			.unwrap_or_default()
			.xmap(fold)
	}

	/// Select the entity whose tree row reads `label`, waiting for its
	/// inspector's title to name it.
	async fn select(&mut self, label: &str) {
		self.find_row(label).await.click().await.unwrap();
		self.find_containing(".scene-inspector-title", label).await;
	}

	/// The selector of the inspector's control for the component `key`, by
	/// the form name the control carries (`entities.2.<type path>`).
	fn control(key: &str) -> String {
		format!(".scene-inspector [name$=\"::{key}\"]")
	}

	/// The inspector's component picker: the one select the form leaves
	/// unnamed, so a key still being chosen is never submitted.
	const COMPONENT_PICKER: &str = ".scene-inspector select:not([name])";

	/// Choose through the picker at `selector` by typing `prefix`: the
	/// browser's own type-ahead on the native select, the twin of the
	/// terminal's type-to-refine.
	async fn pick(&self, selector: &str, prefix: &str) {
		self.find(selector).await.type_ahead(prefix).await.unwrap();
	}

	/// Click the last `Remove` on the inspector: the last component card's.
	async fn remove_last_component(&self) {
		let buttons = self.find_all(".scene-inspector button").await;
		let mut removes = Vec::new();
		for button in buttons {
			if button.text_content().await.unwrap().as_deref() == Some("Remove")
			{
				removes.push(button);
			}
		}
		removes.last().expect("no Remove").click().await.unwrap();
	}

	/// The control holding the keyboard focus and where its caret sits, as
	/// `(name, selectionStart)`.
	async fn caret(&self) -> (String, u64) {
		let caret = self
			.evaluate_value(
				"[document.activeElement.name, document.activeElement.selectionStart]",
			)
			.await
			.unwrap();
		(
			caret[0].as_str().unwrap_or_default().to_string(),
			caret[1].as_u64().unwrap_or_default(),
		)
	}

	/// The footer's lines as the page shows them, the surface fork
	/// (`terminal-only`/`terminal-hidden`) applied: whichever spans are laid
	/// out.
	async fn footer_lines(&self) -> Vec<String> {
		self.evaluate_value(
			"[...document.querySelectorAll('footer span')]
				.filter((span) => getComputedStyle(span).display !== 'none')
				.map((span) => span.textContent)",
		)
		.await
		.unwrap()
		.as_array()
		.unwrap()
		.iter()
		.map(|line| line.as_str().unwrap().to_string())
		.collect()
	}

	/// Whether the browser is marked as holding a fork of this repo.
	async fn is_marked(&self) -> bool {
		self.evaluate_value(&format!("localStorage.getItem({FORK_MARK:?})"))
			.await
			.unwrap()
			.as_str() == Some("1")
	}

	/// The fork as the browser's store holds it, read straight out of
	/// IndexedDB, `None` until the first edit writes it. Probes the database
	/// list first: an `open` creates a missing database, and one created here
	/// without the object store would refuse the world's own opens.
	async fn fork(&self) -> Option<Value> {
		self.evaluate_value(&format!(
			r#"(async () => {{
				const databases = await indexedDB.databases();
				if (!databases.some((db) => db.name === {FORK_DB:?})) return null;
				return new Promise((resolve, reject) => {{
					const open = indexedDB.open({FORK_DB:?});
					open.onerror = () => reject(open.error);
					open.onsuccess = () => {{
						const db = open.result;
						if (!db.objectStoreNames.contains({FORK_OBJECT_STORE:?})) {{
							db.close();
							return resolve(null);
						}}
						const get = db
							.transaction({FORK_OBJECT_STORE:?}, 'readonly')
							.objectStore({FORK_OBJECT_STORE:?})
							.get({FORK_KEY:?});
						get.onerror = () => {{ db.close(); reject(get.error); }};
						get.onsuccess = () => {{
							db.close();
							resolve(get.result ? new TextDecoder().decode(get.result) : null);
						}};
					}};
				}});
			}})()"#
		))
		.await
		.unwrap()
		.as_str()
		.map(|json| serde_json::from_str(json).unwrap())
	}

	/// The fork the browser's store holds, waited for: a write lands off a
	/// task after the edit that made it.
	async fn fork_landed(&self) -> Value {
		poll_ext::poll_async(async || {
			self.fork()
				.await
				.ok_or_else(|| bevyhow!("the fork has not landed"))
		})
		.await
		.unwrap()
	}

	/// The fork as the server's store holds it, what it published, waited
	/// for: the server's first boot writes it off a task.
	async fn published(&self) -> Value {
		let store = &self.store;
		poll_ext::poll_async(async || store.get(&RelPath::from(FORK)).await)
			.await
			.map(|bytes| serde_json::from_slice(&bytes).unwrap())
			.unwrap()
	}

	/// Close the tab and the served app, failing on anything either
	/// reported.
	async fn kill(self) { self.0.kill().await }
}

/// No-break spaces folded to spaces, so a tree needle reads as the terminal
/// paints it.
fn fold(text: String) -> String { text.replace('\u{a0}', " ") }

/// The labels of every entity `scene` holds, in document order.
fn labels(scene: &Value) -> Vec<String> {
	let entities = SceneEntities::of(scene).unwrap();
	entities
		.keys()
		.unwrap()
		.into_iter()
		.map(|key| entities.label(key))
		.collect()
}

/// The page boots as a first boot on both ends: the server builds the
/// original, forks it into its store with its origin recorded and publishes
/// exactly that; the tab paints the published page, editor closed, and when
/// the editor is reached for boots the entry through the published store,
/// adopting the page whole and listing its fifteen entities. The tab's own
/// store stays untouched: a first boot reads upstream and writes nothing.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn a_first_boot_paints_the_published_page() {
	let mut host = SceneHost::new().await;
	host.find_text("carrots").await;
	// closed: the disclosure ships shut
	host.find("details").await;
	host.xpect_no_selector("details[open]").await;
	// the footer's line for this surface shows, and the terminal's does not
	host.footer_lines().await.xpect_eq(vec![
		"Tab moves · Enter activates · ↑↓ scrolls · edits stay in this browser"
			.to_string(),
	]);

	let published = host.published().await;
	labels(&published).xpect_eq(vec![
		"#0 main",
		"#1 h1",
		"#2 \"Garden\"",
		"#3 p",
		"#4 \"Every entity on th…\"",
		"#5 h2",
		"#6 \"Beds\"",
		"#7 ul",
		"#8 li",
		"#9 \"carrots\"",
		"#10 li",
		"#11 \"beets\"",
		"#12 li",
		"#13 \"kale\"",
		"#14 ToggleSceneEditor",
	]);
	// the fork relation, on the first entity
	SceneEntities::of(&published)
		.unwrap()
		.component(0, SceneFork::type_path())
		.unwrap()
		.get_path(&["from".into()])
		.unwrap()
		.as_str()
		.unwrap()
		.xpect_eq(SCENE);

	// open: the boot adopts the served page untouched, and the editor lists
	// the same fifteen entities, nothing selected yet
	host.open_editor().await;
	host.painted_clean("/").await;
	host.rows().await.xpect_eq(vec![
		"#0 main",
		"├ #1 h1",
		"│ └ #2 \"Garden\"",
		"├ #3 p",
		"│ └ #4 \"Every entity on th…\"",
		"├ #5 h2",
		"│ └ #6 \"Beds\"",
		"├ #7 ul",
		"│ ├ #8 li",
		"│ │ └ #9 \"carrots\"",
		"│ ├ #10 li",
		"│ │ └ #11 \"beets\"",
		"│ └ #12 li",
		"│   └ #13 \"kale\"",
		"└ #14 ToggleSceneEditor",
	]);
	host.find_text("Select an entity in the tree").await;
	// nothing written locally: the published fork is what the tab reads
	host.fork().await.xpect_none();
	host.is_marked().await.xpect_false();
	host.kill().await;
}

/// Item 3's loop, through trusted input in the tab: select the heading's
/// text, retype it and watch the heading repaint mid-keystroke with the
/// caret untouched, add a `Name` through the component picker, reparent the
/// paragraph under the heading through its `ChildOf` picker, remove the
/// `Name` through its card, then reload the page and find every edit came
/// back from the fork in the browser's own store.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn the_edit_loop_survives_a_reload() {
	let mut host = SceneHost::new().await;
	host.open_editor().await;

	// a text edit reaches the page mid-keystroke: the heading, the tree row
	// and the inspector title follow each key, and the control keeps focus
	// and caret
	host.select("#2 \"Garden\"").await;
	let value = SceneHost::control("Value");
	let control = host.find(&value).await;
	control.xpect_value("Garden").await;
	let name = control.get_attribute("name").await.unwrap().unwrap();
	control.type_text(" Par").await.unwrap();
	host.find("h1").await.xpect_text("Garden Par").await;
	host.find_row("#2 \"Garden Par\"").await;
	host.find_containing(".scene-inspector-title", "#2 \"Garden Par\"")
		.await;
	host.caret()
		.await
		.xpect_eq((name.clone(), "Garden Par".len() as u64));
	// the same control, not a regenerated one
	control.type_text("ty").await.unwrap();
	host.find("h1").await.xpect_text("Garden Party").await;
	host.find_row("│ └ #2 \"Garden Party\"").await;
	host.caret()
		.await
		.xpect_eq((name, "Garden Party".len() as u64));
	// the first edit forked the page into the browser's store
	let fork = host.fork_landed().await;
	labels(&fork)[2].as_str().xpect_eq("#2 \"Garden Party\"");
	host.is_marked().await.xpect_true();

	// a component added through the picker: its zero lands on the entity,
	// and its control names the row
	host.pick(SceneHost::COMPONENT_PICKER, "Name").await;
	host.find(SceneHost::COMPONENT_PICKER)
		.await
		.xpect_value(Name::type_path())
		.await;
	host.click_text("Add component").await.unwrap();
	let name_control = host.find(&SceneHost::control("Name")).await;
	name_control.xpect_value("").await;
	name_control.type_text("Heading").await.unwrap();
	host.find_row("│ └ Heading").await;

	// a reparent through the `ChildOf` picker: the paragraph moves under the
	// heading on the page and in the tree
	host.select("#3 p").await;
	host.pick(&SceneHost::control("ChildOf"), "#1").await;
	host.find_row("│ └ #3 p").await;
	host.find_row("│ ├ Heading").await;
	host.find_row("│   └ #4 \"Every entity on th…\"").await;
	host.find("h1 > p").await;

	// a component removed through its card: the row reads by key again
	host.select("Heading").await;
	host.remove_last_component().await;
	host.find_row("│ ├ #2 \"Garden Party\"").await;
	host.xpect_no_selector(&SceneHost::control("Name")).await;
	// the fork holds every edit, and the server's published page none
	let edited = poll_ext::poll_async(async || {
		let fork = host.fork_landed().await;
		let landed = {
			let entities = SceneEntities::of(&fork)?;
			entities.target(3, ChildOf::type_path()) == Some(1)
				&& entities.component(2, Name::type_path()).is_none()
		};
		landed
			.then_some(fork)
			.ok_or_else(|| bevyhow!("the last edit has not landed"))
	})
	.await
	.unwrap();
	labels(&host.published().await)[2]
		.as_str()
		.xpect_eq("#2 \"Garden\"");

	// the next session boots the edited page from the fork
	host.reload().await;
	host.find("h1")
		.await
		.xpect_contains_text("Garden Party")
		.await;
	host.find("h1 > p").await;
	host.fork().await.xpect_eq(Some(edited));
	host.open_editor().await;
	host.find_row("│ ├ #2 \"Garden Party\"").await;
	host.find_row("│ └ #3 p").await;
	host.kill().await;
}

/// A child added under the last sibling indents under it: its guides open
/// two cells deeper than its parent's, and it is selected as it lands with
/// its parent shown in its `ChildOf` picker.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn an_added_child_indents_under_its_parent() {
	let mut host = SceneHost::new().await;
	host.open_editor().await;
	host.select("#14 ToggleSceneEditor").await;
	host.click_text("Add child").await.unwrap();
	host.find_containing(".scene-inspector-title", "#15").await;
	host.find_row("#15")
		.await
		.xpect_attr("aria-selected", "true")
		.await;
	let parent = host.guides("#14 ToggleSceneEditor").await;
	host.guides("#15")
		.await
		.chars()
		.count()
		.xpect_eq(parent.chars().count() + 2);
	host.find(&SceneHost::control("ChildOf"))
		.await
		.xpect_value("14")
		.await;
	SceneEntities::of(&host.fork_landed().await)
		.unwrap()
		.target(15, ChildOf::type_path())
		.xpect_eq(Some(14));
	host.kill().await;
}

/// The editor is an entity of the page it edits: removing it through its own
/// inspector is an ordinary edit, taking the editor ui with it, persisted
/// like any other, so the next load boots a page without one.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn removing_the_editor_is_an_ordinary_edit() {
	let mut host = SceneHost::new().await;
	host.open_editor().await;
	host.select("#14 ToggleSceneEditor").await;
	// the header's `Remove` is the first on the page
	host.click_text("Remove").await.unwrap();
	host.xpect_no_selector("summary").await;
	host.xpect_no_selector(".scene-tree-row").await;
	host.find_text("carrots").await;
	let fork = host.fork_landed().await;
	labels(&fork).len().xpect_eq(14);

	// the next session boots from the fork: the served page still carries
	// the editor, the world painting over it does not
	host.reload().await;
	host.find_text("carrots").await;
	host.xpect_no_selector("summary").await;
	host.fork().await.xpect_eq(Some(fork));
	host.kill().await;
}

/// A click before the wasm has booted lands after it: opening the editor is
/// the intent that loads the wasm, a row clicked while it loads is queued by
/// the pre-boot script, and once the world has adopted the served page (with
/// nothing replaced or patched) the replay selects the row and the inspector
/// generates for it.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn a_pre_boot_click_lands_after_boot() {
	let mut host = SceneHost::new().await;
	// the served page is inert: the disclosure is the browser's, and its
	// toggle is what injects the loader
	host.click("summary").await.unwrap();
	host.find_row("#2 \"Garden\"").await.click().await.unwrap();
	// both clicks are queued: the wasm is still on its way
	host.evaluate_value(&format!(
		"globalThis.{}?.q.length ?? -1",
		PreBoot::GLOBAL
	))
	.await
	.unwrap()
	.as_i64()
	.xpect_eq(Some(2));
	host.painted_clean("/").await;
	// the replay: the row is selected and the inspector generated for it
	host.find_containing(".scene-inspector-title", "#2 \"Garden\"")
		.await;
	host.find(".scene-tree-row[aria-selected=\"true\"]")
		.await
		.xpect_contains_text("#2 \"Garden\"")
		.await;
	host.kill().await;
}

/// Booting is invisible: the served page and the page the world paints are
/// the same document, pixel for pixel and node for node.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn booting_is_invisible() {
	let mut host = SceneHost::new().await;
	let before = host.screenshot().await.unwrap();
	let html_before = host
		.evaluate_value("document.body.innerHTML")
		.await
		.unwrap();
	// the intent without the gesture, so the page is exactly as served when
	// the world adopts it
	host.evaluate(
		"document.querySelector('details').dispatchEvent(new Event('toggle'))",
	)
	.await
	.unwrap();
	host.painted_clean("/").await;
	let after = host.screenshot().await.unwrap();
	let html_after = host
		.evaluate_value("document.body.innerHTML")
		.await
		.unwrap();
	html_after.xpect_eq(html_before);
	(after == before).xpect_true();
	host.kill().await;
}

/// A returning editor never sees the published text: an edit forks the page
/// into the browser's store and marks the browser, and the next load hides
/// the served body and boots at once, revealing the page only once the fork
/// has painted over it.
#[beet_core::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-ui` + chromedriver"]
async fn a_returning_editor_never_sees_the_published_text() {
	let mut host = SceneHost::new().await;
	host.open_editor().await;
	// the edit: the heading's text, retyped through its inspector
	host.select("#2 \"Garden\"").await;
	host.find(&SceneHost::control("Value"))
		.await
		.type_text("!")
		.await
		.unwrap();
	host.find("h1").await.xpect_text("Garden!").await;
	// the fork lands in the browser's store, marking it as an editor's
	poll_ext::poll_async(async || match host.is_marked().await {
		true => Ok(()),
		false => bevybail!("the fork is not marked yet"),
	})
	.await
	.unwrap();
	// the next load: hidden, and booting without a gesture
	host.goto("/").await;
	let mut samples = 0;
	host.painted_with("/", async |page| {
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
		(visibility == "hidden" || heading == "Garden!").xpect_true();
	})
	.await;
	samples.xpect_greater_than(0);
	// revealed, and the fork is what shows
	host.find("h1").await.xpect_text("Garden!").await;
	host.evaluate_value("getComputedStyle(document.body).visibility")
		.await
		.unwrap()
		.as_str()
		.xpect_eq(Some("visible"));
	host.xpect_no_selector(&format!("#{}", PreBoot::STYLE_ID))
		.await;
	host.kill().await;
}
