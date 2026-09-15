//! The live-TUI acceptance suite for the on-disk `examples/ui/scene_editor.bsx`
//! page: the real entry and the scene it forks, booted into an in-process
//! [`ChannelTerminal`] and driven with terminal bytes.
//!
//! This is the scene editor's embodiment gate. Every widget is pinned in its
//! own harness; what only this suite can show is that an *authored* entry
//! assembles them: a page forked into a scene document on first boot, the
//! editor the page carries as one of its own entities, every control editing
//! that document with the live page following, and a reboot from the store
//! reproducing the edited page.
//!
//! The store is in-memory, seeded from the on-disk entry through the shared
//! [`TuiHost`] without the fork, so every case is a first boot and a reboot
//! over the same store is the persistence case.
beet::test_main!();

use beet::prelude::*;
use std::ops::Deref;
use std::ops::DerefMut;

#[path = "tui_host/mod.rs"]
mod tui_host;
use tui_host::TuiHost;

/// The entry document within the examples directory.
const ENTRY: &str = "scene_editor.bsx";
/// The authored original of the scene, as the entry names it.
const SCENE: &str = "scene_editor/scene.bsx";
/// The document shell the entry wraps its page in, under its `<TemplateDir>`.
const SHELL: &str = "templates/EditorShell.bsx";
/// The fork, as the entry names it: absent until the first boot writes it.
const FORK: &str = "scene_editor/scene.json";
/// A viewport tall enough for the page and the open editor, so no case
/// scrolls.
const SIZE: UVec2 = UVec2::new(120, 60);

/// The booted page: the shared [`TuiHost`] over the scene editor entry, plus
/// the scene reads and the editor gestures its cases make.
struct SceneHost(TuiHost);

impl Deref for SceneHost {
	type Target = TuiHost;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for SceneHost {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl SceneHost {
	/// Boot the page over a store holding the entry and the authored original
	/// alone, so the boot is a first boot, and wait for the page to paint.
	async fn new() -> Self {
		let store = TuiHost::seeded_store(&[ENTRY, SCENE, SHELL]).await;
		Self::boot(store).await
	}

	/// Boot the page over `store` and wait for it to paint.
	async fn boot(store: BlobStore) -> Self {
		let mut host = Self(TuiHost::boot(store, ENTRY, SIZE).await);
		host.step_until("carrots");
		host
	}

	/// Let every write-back land, then boot a fresh app over the same store:
	/// the next session, reading the fork this one wrote.
	async fn reboot(mut self) -> Self {
		AsyncRunner::settle_async_tasks(self.app.world_mut()).await;
		Self::boot(self.0.store).await
	}

	/// The scene document's host, the `<SceneBlob>` the scene landed on.
	fn scene(&mut self) -> Entity {
		self.app
			.world_mut()
			.query_filtered::<Entity, With<SceneDocument>>()
			.single(self.app.world())
			.expect("no scene document")
	}

	/// The scene document as the live app holds it.
	fn document(&mut self) -> Value {
		let scene = self.scene();
		self.app.world().get::<Document>(scene).unwrap().0.clone()
	}

	/// The live entity the scene's `key` built into.
	fn live(&mut self, key: u32) -> Entity {
		let scene = self.scene();
		self.app
			.world()
			.get::<TemplateEntityMap>(scene)
			.unwrap()
			.world(key)
			.unwrap_or_else(|| panic!("entity #{key} is not live"))
	}

	/// The fork as the store holds it: the record of the session.
	async fn fork(&self) -> Value {
		let bytes = self.store.get(&RelPath::from(FORK)).await.unwrap();
		serde_json::from_slice(&bytes).unwrap()
	}

	/// Open the editor's disclosure and wait for its tree.
	fn open_editor(&mut self) {
		self.click_text("Scene editor");
		self.step_until("#14 ToggleSceneEditor");
	}

	/// Select the entity whose tree row reads `label`, waiting for its
	/// inspector: the title is the label's second sighting, and the generated
	/// controls bind their values a frame after the form they sit in.
	fn select(&mut self, label: &str) {
		self.click_text(label);
		self.step_until_count(label, 2);
		self.step_until("Add component");
		self.settle(8);
	}

	/// Choose through the picker keyed `key`: open it, type `filter` to refine
	/// its rows, and Enter the first match.
	fn pick(&mut self, key: &str, filter: &str) {
		self.click_control_of(key, 0);
		self.type_text(filter);
		self.press_enter();
	}
}

/// The page boots as a first boot: the original is built and painted, the
/// editor ships closed, and the fork written to the store is exactly the
/// authored page with its origin recorded, never the editor ui.
#[beet::test]
async fn a_first_boot_forks_the_page() {
	let mut host = SceneHost::new().await;
	host.folded_frame()
		.xpect_contains("Garden")
		.xpect_contains("carrots")
		// closed: the tree is not on the page
		.xnot()
		.xpect_contains("#0 main");

	let fork = host.fork().await;
	let entities = SceneEntities::of(&fork).unwrap();
	entities
		.keys()
		.unwrap()
		.into_iter()
		.map(|key| entities.label(key))
		.collect::<Vec<_>>()
		.xpect_eq(vec![
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
	entities
		.component(0, SceneFork::type_path())
		.unwrap()
		.get_path(&["from".into()])
		.unwrap()
		.as_str()
		.unwrap()
		.xpect_eq(SCENE);

	// open, the editor lists the same fifteen entities, and nothing yet
	host.open_editor();
	host.frame()
		.xpect_contains("#0 main")
		.xpect_contains("│   └ #13 \"kale\"")
		.xpect_contains("Select an entity in the tree");
}

/// Item 3's loop, through the terminal: select the heading's text, retype it
/// and watch the heading repaint mid-keystroke with the caret untouched, add a
/// `Name` through the component picker, reparent the paragraph under the
/// heading through its `ChildOf` picker, remove the `Name` through its card,
/// then reboot from the store and find every edit came back from the fork.
#[beet::test]
async fn the_edit_loop_survives_a_reboot() {
	let mut host = SceneHost::new().await;
	host.open_editor();

	// a text edit reaches the page mid-keystroke: the heading, the tree row and
	// the inspector title follow each key, and the control keeps focus and caret
	host.select("#2 \"Garden\"");
	host.click_control_of("Value", 0);
	let control = host.focused();
	host.type_text(" Par");
	host.step_until_folded("Garden Par")
		.xpect_contains("#2 \"Garden Par\"")
		.xpect_contains("Garden Par\u{258f}");
	host.focused().xpect_eq(control);
	host.type_text("ty");
	host.step_until_folded("Garden Party")
		.xpect_contains("│ └ #2 \"Garden Party\"")
		.xpect_contains("Garden Party\u{258f}");
	host.focused().xpect_eq(control);

	// a component added through the picker: its zero lands on the live
	// entity, and its control names the row
	host.pick("Component", "name");
	host.click_text("Add component");
	// a fourth `Remove`: the header's, the two cards', and the new card's
	host.step_until_count("Remove", 4);
	let heading = host.live(2);
	host.app
		.world()
		.get::<Name>(heading)
		.unwrap()
		.as_str()
		.xpect_eq("");
	host.click_control_of("Name", 0);
	host.type_text("Heading");
	host.step_until("│ └ Heading");
	host.app
		.world()
		.get::<Name>(heading)
		.unwrap()
		.as_str()
		.xpect_eq("Heading");

	// a reparent through the `ChildOf` picker: the paragraph moves under the
	// heading on the page and in the tree
	host.select("#3 p");
	host.pick("ChildOf", "h1");
	host.step_until("│ └ #3 p")
		.xpect_contains("│ ├ Heading")
		.xpect_contains("│   └ #4 \"Every entity on th…\"");
	let (h1, paragraph) = (host.live(1), host.live(3));
	host.app
		.world()
		.get::<ChildOf>(paragraph)
		.unwrap()
		.parent()
		.xpect_eq(h1);

	// a component removed through its card: the row reads by key again
	host.select("Heading");
	host.click_last("Remove");
	host.step_until("│ ├ #2 \"Garden Party\"");
	host.app.world().get::<Name>(heading).xpect_none();
	let edited = host.document();

	// the next session boots the edited page from the fork
	let mut host = host.reboot().await;
	host.folded_frame().xpect_contains("Garden Party");
	host.document().xpect_eq(edited);
	let (h1, paragraph, heading) = (host.live(1), host.live(3), host.live(2));
	host.app
		.world()
		.get::<ChildOf>(paragraph)
		.unwrap()
		.parent()
		.xpect_eq(h1);
	host.app.world().get::<Name>(heading).xpect_none();
	host.open_editor();
	host.frame()
		.xpect_contains("│ ├ #2 \"Garden Party\"")
		.xpect_contains("│ └ #3 p");
}

/// A child added under the last sibling indents under it: its guides open
/// with no-break spaces the terminal keeps, and it is selected as it lands.
#[beet::test]
async fn an_added_child_indents_under_its_parent() {
	let mut host = SceneHost::new().await;
	host.open_editor();
	host.select("#14 ToggleSceneEditor");
	host.click_text("Add child");
	host.step_until_count("#15", 2);
	// the column each row's corner glyph paints at
	let corner = |host: &SceneHost, label: &str| host.cell_of_nth(label, 0).0;
	corner(&host, "└ #15")
		.xpect_eq(corner(&host, "└ #14 ToggleSceneEditor") + 2);
	let (parent, child) = (host.live(14), host.live(15));
	host.app
		.world()
		.get::<ChildOf>(child)
		.unwrap()
		.parent()
		.xpect_eq(parent);
}

/// The editor is an entity of the page it edits: removing it through its own
/// inspector is an ordinary edit, taking the editor ui with it, persisted like
/// any other.
#[beet::test]
async fn removing_the_editor_is_an_ordinary_edit() {
	let mut host = SceneHost::new().await;
	host.open_editor();
	host.select("#14 ToggleSceneEditor");
	// the header's `Remove` is the first on the page
	host.click_text("Remove");
	host.settle(8);
	host.frame()
		.xnot()
		.xpect_contains("Scene editor")
		.xnot()
		.xpect_contains("#0 main");
	SceneEntities::of(&host.document())
		.unwrap()
		.contains(14)
		.xpect_false();

	let host = host.reboot().await;
	host.frame().xnot().xpect_contains("Scene editor");
	SceneEntities::of(&host.fork().await)
		.unwrap()
		.keys()
		.unwrap()
		.len()
		.xpect_eq(14);
}
