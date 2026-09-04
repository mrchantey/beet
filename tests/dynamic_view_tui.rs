//! The live-TUI acceptance suite for the on-disk `examples/ui/dynamic_view.bsx`
//! todo app: the real entry and its two JSON documents, booted into an in-process
//! [`ChannelTerminal`] and driven with terminal bytes.
//!
//! This is the embodiment gate. Every piece the app is made of is unit-tested in
//! its own crate; what only this suite can show is that an *authored* entry
//! assembles them: a schema document and a data document read out of a store, a
//! view and a form generated from what those documents declare, and edit mode
//! evolving the schema and the rows together while the terminal repaints.
//!
//! The store is in-memory, seeded from the on-disk entry, so the suite exercises
//! the same load and the same write-back without rewriting the example's own
//! documents.
beet::test_main!();

use beet::prelude::*;

/// The entry's directory in the repo, the store this suite seeds itself from.
const ENTRY_DIR: &str = "examples/ui";
/// The entry document within it.
const ENTRY: &str = "dynamic_view.bsx";
/// The data document, as the entry names it.
const TODOS: &str = "dynamic_view/todos.json";
/// The schema document, as the entry names it.
const SCHEMA: &str = "dynamic_view/schema.json";

/// A booted live-TUI todo app: the on-disk entry built into a router root, a
/// page host and an in-world navigator, driven through a channel terminal.
struct TodoHost {
	app: App,
	host: Entity,
	/// The in-memory store the entry was built from, so a test can read back
	/// what an edit persisted.
	store: BlobStore,
}

/// An SGR mouse sequence: button `button` at 0-indexed cell `(col, row)`,
/// pressed (`M`) or released (`m`).
fn sgr(button: u32, col: u32, row: u32, pressed: bool) -> Vec<u8> {
	let suffix = if pressed { 'M' } else { 'm' };
	format!("\x1b[<{button};{};{}{suffix}", col + 1, row + 1).into_bytes()
}

/// The entry and its documents, copied off disk into an in-memory store: the
/// suite drives the real markup and the real documents without the app's
/// write-back landing in the repo.
async fn seeded_store() -> BlobStore {
	let disk = BlobStore::new(FsStore::new(
		AbsPathBuf::new_workspace_rel(ENTRY_DIR).unwrap(),
	));
	let store = BlobStore::temp();
	for path in [ENTRY, TODOS, SCHEMA] {
		let path = SmolPath::from(path);
		let bytes = disk.get(&path).await.unwrap();
		store.insert(&path, bytes).await.unwrap();
	}
	store
}

impl TodoHost {
	/// Boot the app at a `size`-cell viewport: build the entry onto a root
	/// carrying the seeded store (so `<DocumentBlob>` resolves it by ancestry),
	/// settle the async reads, then pair a channel terminal with a page host and
	/// an in-world navigator, exactly as the binary's `--server=tui` boot does.
	async fn new(size: UVec2) -> Self {
		let mut app = App::new();
		app.add_plugins((
			RouterPlugin,
			CharcellTuiPlugin,
			NavigatorPlugin,
			LivePagePlugin,
			material::MaterialStylePlugin,
		))
		.insert_resource(pkg_config!());

		let store = seeded_store().await;
		let entry = store.get_media(&SmolPath::from(ENTRY)).await.unwrap();
		let source = entry.as_utf8().unwrap();
		let root = app.world_mut().spawn_empty().id();
		let template =
			BsxTemplate::parse_entry(app.world_mut(), source).unwrap();
		app.world_mut()
			.entity_mut(root)
			.insert(store.clone())
			.insert_template(template)
			.unwrap();
		// the store reads that answer the two documents are tasks, so the tree
		// exists frames before they do; settling here is the boot, not a fixup.
		AsyncRunner::settle_async_tasks(app.world_mut()).await;

		let router = app
			.world_mut()
			.run_system_cached_with::<_, Result<Entity>, _, _>(
				find_router,
				root,
			)
			.unwrap()
			.unwrap();
		let (channel, terminal) =
			ChannelTerminal::new(TerminalConfig::default());
		let host = app
			.world_mut()
			.spawn((
				channel,
				terminal,
				PageHost::bundle(size),
				Navigator::in_world(router, "/"),
				// deterministic frames whatever terminal runs the tests
				KittyGraphicsSupport { enabled: false },
			))
			.id();
		app.update();
		Self { app, host, store }
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

	/// The 0-indexed start cell of the `nth` (0-based) occurrence of `text`,
	/// scanning the frame top to bottom. Several generated controls share a
	/// label (every collection has an `add`), so a click names which one.
	fn cell_of_nth(&self, text: &str, nth: usize) -> (u32, u32) {
		let mut seen = 0;
		for (row, line) in self.frame().lines().enumerate() {
			let mut from = 0;
			while let Some(col) = line[from..].find(text) {
				let col = from + col;
				if seen == nth {
					return (col as u32, row as u32);
				}
				seen += 1;
				from = col + text.len();
			}
		}
		panic!(
			"text {text:?} occurs fewer than {} times in frame:\n{}",
			nth + 1,
			self.frame()
		);
	}

	/// Click (press + release) the cell at `(col, row)`.
	fn click(&mut self, col: u32, row: u32) {
		self.send(&sgr(0, col, row, true));
		self.app.update();
		self.send(&sgr(0, col, row, false));
		self.app.update();
	}

	/// Click the `nth` occurrence of `text`.
	fn click_nth(&mut self, text: &str, nth: usize) {
		let (col, row) = self.cell_of_nth(text, nth);
		self.click(col, row);
	}

	/// Click the first occurrence of `text`.
	fn click_text(&mut self, text: &str) { self.click_nth(text, 0) }

	/// Click the last occurrence of `text`.
	fn click_last(&mut self, text: &str) {
		// a zero count falls through to `cell_of_nth`, whose panic names the
		// missing text and prints the frame
		let count = self.frame().matches(text).count();
		self.click_nth(text, count.saturating_sub(1));
	}

	/// Advance `frames` frames, for a settle with no distinctive needle.
	fn settle(&mut self, frames: usize) {
		for _ in 0..frames {
			self.app.update();
		}
	}

	/// Type `text` into the focused element, as a terminal delivers it.
	fn type_text(&mut self, text: &str) {
		self.send(text.as_bytes());
		self.settle(8);
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
		self.step_until_count(needle, 1)
	}

	/// Advance frames until the frame contains `needle` at least `count` times,
	/// returning the frame. A generated widget and the draft that produced it
	/// carry the same text, so "the table grew the column" is a second
	/// occurrence rather than a first.
	fn step_until_count(&mut self, needle: &str, count: usize) -> String {
		for _ in 0..200 {
			self.app.update();
			let frame = self.frame();
			if frame.matches(needle).count() >= count {
				return frame;
			}
		}
		panic!(
			"frame never contained '{needle}' {count} times:\n{}",
			self.frame()
		);
	}

	/// The document persisted at `path` in the app's own store.
	///
	/// Read through a default registry, whose intrinsic meta-schema is what a
	/// *schema* document's own declaration resolves through; the row schema the
	/// data document names is unregistered here and defers to a wildcard, which
	/// is the read this assertion wants.
	async fn stored(&self, path: &str) -> TypedDocument {
		let registry = SchemaRegistry::default();
		self.store
			.get_document(
				SchemaResolver::default().with_schemas(&registry),
				&SmolPath::from(path),
			)
			.await
			.unwrap()
	}
}

/// The app boots: the data document is read out of the store, the row schema out
/// of the schema document beside it, and the view generated from the pair paints
/// a column per field and a row per todo.
#[beet::test]
async fn boots_and_paints_the_rows() {
	let mut host = TodoHost::new(UVec2::new(120, 48)).await;
	let frame = host.step_until("buy milk");
	// the columns are the row schema's own fields, named by nothing in the entry
	frame.as_str().xpect_contains("label");
	frame.as_str().xpect_contains("done");
	frame.xpect_contains("walk dog");
}

/// Editing a row through the generated form reaches the document, the view bound
/// to the same field, and the store: the write-back is what makes an edit outlive
/// the process.
#[beet::test]
async fn an_edit_reaches_the_view_and_the_store() {
	let mut host = TodoHost::new(UVec2::new(120, 48)).await;
	host.step_until("buy milk");
	// the second "buy milk" is the form's control; the first is the view's cell
	let (col, row) = host.cell_of_nth("buy milk", 1);
	host.click(col + 1, row);
	host.type_text(" and eggs");
	// the read-only view is bound to the same field, so it reflows
	host.step_until("buy milk and eggs");

	host.stored(TODOS).await.value.xpect_eq(value!([
		{ "label": "buy milk and eggs", "done": true },
		{ "label": "walk dog", "done": false },
	]));
}

/// Edit mode is opt-in (item 11): the schema editor ships with the app but stays
/// behind a closed disclosure until something opens it.
#[beet::test]
async fn edit_mode_is_opt_in() {
	let mut host = TodoHost::new(UVec2::new(120, 220)).await;
	host.step_until("buy milk");
	// closed: the editor's commit button is not on the page
	host.frame().xnot().xpect_contains("Apply");
	host.click_text("Edit schema");
	// open: the meta-schema form and its commit are
	host.step_until("Apply").xpect_contains("StructSchema");
}

/// Item 3's acceptance loop, through the terminal: open edit mode, add a bool
/// field to the *row schema*, apply, and the table has an extra column and the
/// form an extra control — both because they are generated from the schema that
/// just changed, and neither because anything here knows what a todo is.
#[beet::test]
async fn adding_a_field_grows_the_table_and_the_form() {
	let mut host = TodoHost::new(UVec2::new(120, 260)).await;
	host.step_until("buy milk");
	host.click_text("Edit schema");
	host.step_until("Apply");

	// the row schema's `fields` list is the last collection on the page, so its
	// add button is the last one; it appends the field schema's own zero
	host.click_last("add");
	host.settle(8);
	// name the field, in the empty control the appended row generated
	let (col, row) = host.cell_of_nth("key", 2);
	host.click(col + 4, row + 1);
	host.type_text("is_really_difficult");
	// ...and type it, through the variant select the meta-schema's own enum
	// generated for the field's `schema`
	host.click_text("Any \u{25be}");
	host.settle(8);
	host.click_last("Bool");
	host.settle(8);
	// the drafted field is optional, which is one of item 21's resolutions: the
	// rows that already exist stay valid without a backfill
	host.click_text("Apply");

	// the table generated from the committed schema grew the column, beside the
	// draft that named it...
	host.step_until_count("is_really_difficult", 2);
	// ...and every row survived it. The regenerated rows bind their values a
	// frame after the layout they sit in, so this is a second wait, not the
	// same frame.
	host.step_until("buy milk").xpect_contains("walk dog");

	// the commit is transactional across the pair, and both halves persisted
	host.stored(SCHEMA)
		.await
		.to_schema()
		.unwrap()
		.get_field_schema(&FieldPath::new(["is_really_difficult"]))
		.unwrap()
		.xpect_eq(ValueSchema::Bool(default()));
	host.stored(TODOS).await.value.xpect_eq(value!([
		{ "label": "buy milk", "done": true },
		{ "label": "walk dog", "done": false },
	]));
}
