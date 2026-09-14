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
//! The store is in-memory, seeded from the on-disk entry through the shared
//! [`TuiHost`], so the suite exercises the same load and the same write-back
//! without rewriting the example's own documents.
beet::test_main!();

use beet::prelude::*;
use std::ops::Deref;
use std::ops::DerefMut;

#[path = "tui_host/mod.rs"]
mod tui_host;
use tui_host::TuiHost;

/// The entry document within the examples directory.
const ENTRY: &str = "dynamic_view.bsx";
/// The data document, as the entry names it.
const TODOS: &str = "dynamic_view/todos.json";
/// The schema document, as the entry names it.
const SCHEMA: &str = "dynamic_view/schema.json";

/// The booted todo app: the shared [`TuiHost`] over the todo entry and its two
/// JSON documents, plus the reads and outside edits its cases make.
struct TodoHost(TuiHost);

impl Deref for TodoHost {
	type Target = TuiHost;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for TodoHost {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl TodoHost {
	/// Boot the app at a `size`-cell viewport over the entry and its two
	/// documents, copied off disk.
	async fn new(size: UVec2) -> Self {
		let store = TuiHost::seeded_store(&[ENTRY, TODOS, SCHEMA]).await;
		Self(TuiHost::boot(store, ENTRY, size).await)
	}

	/// The entity carrying the data document, ie the `<DocumentBlob>` the
	/// todos were read onto.
	fn todos(&mut self) -> Entity {
		self.app
			.world_mut()
			.query::<(Entity, &DocumentBlob)>()
			.iter(self.app.world())
			.find(|(_, blob)| blob.path.as_str() == TODOS)
			.map(|(entity, _)| entity)
			.expect("no todos document")
	}

	/// How many rows the live data document holds.
	fn row_count(&mut self) -> usize {
		let todos = self.todos();
		self.app
			.world()
			.get::<Document>(todos)
			.unwrap()
			.0
			.as_list()
			.map(Vec::len)
			.unwrap_or_default()
	}

	/// Append a row to the data document from outside the form, the path an
	/// edit nobody's control made takes: a script, a sync, another session.
	fn push_row(&mut self, row: Value) {
		let todos = self.todos();
		self.app
			.world_mut()
			.run_system_once_with::<_, In<Value>, Result, _>(
				move |row: In<Value>, mut docs: DocumentQuery| -> Result {
					let row = row.0;
					docs.with_field(
						todos,
						&FieldRef::default(),
						move |list| {
							list.as_list_mut_or_init()
								.map(|list| list.push(row))
						},
					)??;
					OK
				},
				row,
			)
			.unwrap()
			.unwrap();
		self.settle(8);
	}

	/// The `row`th todo's label, read out of the seeded document.
	///
	/// Every needle a case aims at is derived from this rather than written
	/// down, because the two JSON documents are the example's *living* state
	/// (item 126): running the example rewrites them, so an assertion naming
	/// "buy milk" fails on any tree where someone has used the app.
	async fn seeded_label(&self, row: usize) -> String {
		Document::new(self.stored(TODOS).await.value)
			.get_field(&[FieldSegment::index(row), "label".into()])
			.unwrap()
	}
}

/// The app boots: the data document is read out of the store, the row schema out
/// of the schema document beside it, and the view generated from the pair paints
/// a column per field and a row per todo.
#[beet::test]
async fn boots_and_paints_the_rows() {
	let mut host = TodoHost::new(UVec2::new(120, 48)).await;
	let (first, second) =
		(host.seeded_label(0).await, host.seeded_label(1).await);
	let frame = host.step_until(&first);
	// the columns are the row schema's own fields, named by nothing in the entry
	// (read back humanised, since a key is an identifier and a header is not)
	frame.as_str().xpect_contains("Label");
	frame.as_str().xpect_contains("Done");
	frame.xpect_contains(&second);
}

/// Editing a row through the generated form reaches the document, the view bound
/// to the same field, and the store: the write-back is what makes an edit outlive
/// the process.
#[beet::test]
async fn an_edit_reaches_the_view_and_the_store() {
	let mut host = TodoHost::new(UVec2::new(120, 48)).await;
	// what the document held before the edit, so every assertion below states
	// what the edit *changed* rather than restating the living document
	let mut expected = Document::new(host.stored(TODOS).await.value);
	let label = host.seeded_label(0).await;
	let edited = format!("{label} and eggs");
	host.step_until(&label);
	// the second occurrence is the form's control; the first is the view's cell
	let (col, row) = host.cell_of_nth(&label, 1);
	host.click(col + 1, row);
	host.type_text(" and eggs");
	// the read-only view is bound to the same field, so it reflows
	host.step_until(&edited);

	*expected
		.get_field_mut(&[FieldSegment::index(0), "label".into()])
		.unwrap() = Value::new(edited);
	host.stored(TODOS).await.value.xpect_eq(expected.0);
}

/// A row's control keeps its focus and caret while an append lands beside it
/// from outside the form: the rows are reconciled by key, so only the new row
/// is built and the one being typed into is the same entity before and after.
#[beet::test]
async fn an_append_leaves_a_focused_control_alone() {
	let mut host = TodoHost::new(UVec2::new(120, 80)).await;
	let label = host.seeded_label(0).await;
	host.step_until(&label);
	// the second occurrence is the form's control; the first is the view's cell
	let (col, row) = host.cell_of_nth(&label, 1);
	host.click(col + 1, row);
	host.type_text(" and");
	// the caret paints after the typed text, in the focused control alone
	let typed = format!("{label} and\u{258f}");
	host.step_until(&typed);
	let control = host.focused();
	let rows = host.row_count();

	host.push_row(value!({ "label": "feed the cat", "done": false }));
	host.step_until("feed the cat");
	host.row_count().xpect_eq(rows + 1);
	// the control is the same entity, still focused, its caret untouched
	host.focused().xpect_eq(control);
	host.frame().xpect_contains(&typed);
	// ...so typing carries on where it left off: the view's cell reads the
	// whole label (the control wraps it), and the caret still trails the text
	host.type_text(" eggs");
	host.step_until(&format!("{label} and eggs"))
		.xpect_contains("eggs\u{258f}");
}

/// Clicking the add button follows the normal focus rules: the click blurs the
/// control and focuses the button, and the button survives the append it
/// caused, so pressing Enter appends again with nothing re-focused.
#[beet::test]
async fn the_add_button_survives_its_own_append() {
	let mut host = TodoHost::new(UVec2::new(120, 80)).await;
	let label = host.seeded_label(0).await;
	host.step_until(&label);
	let (col, row) = host.cell_of_nth(&label, 1);
	host.click(col + 1, row);
	host.settle(4);
	let control = host.focused();
	let rows = host.row_count();

	host.click_text("Add item");
	host.settle(8);
	host.row_count().xpect_eq(rows + 1);
	let button = host.focused();
	button.xpect_not_eq(control);

	// Enter activates the focused button: the same one, still focused
	host.press_enter();
	host.row_count().xpect_eq(rows + 2);
	host.focused().xpect_eq(button);
}

/// Edit mode is opt-in (item 11): the schema editor ships with the app but stays
/// behind a closed disclosure until something opens it.
#[beet::test]
async fn edit_mode_is_opt_in() {
	let mut host = TodoHost::new(UVec2::new(120, 220)).await;
	let label = host.seeded_label(0).await;
	host.step_until(&label);
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
	// the living rows, so the survival assertion below says "unchanged" rather
	// than restating a document the example rewrites (item 126)
	let rows = host.stored(TODOS).await.value;
	let (first, second) =
		(host.seeded_label(0).await, host.seeded_label(1).await);
	host.step_until(&first);
	host.click_text("Edit schema");
	host.step_until("Apply");

	// each collection's add button names it, so the row schema's `fields` list is
	// addressed by name rather than by being the last one on the page; it appends
	// the field schema's own zero
	host.click_text("Add to Fields");
	host.settle(8);
	// name the field, in the empty control the appended row generated. `Key` is
	// the humanised label the generated form shows for the `key` field.
	host.click_control_of("Key", 2);
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

	// the table generated from the committed schema grew the column. The header
	// is the key made readable, so it is a *different* string from the one still
	// sitting in the draft's `key` control, which is what makes it evidence the
	// column exists rather than a second sighting of what was typed.
	host.step_until("Is really difficult");
	// ...and every row survived it. The regenerated rows bind their values a
	// frame after the layout they sit in, so this is a second wait, not the
	// same frame.
	host.step_until(&first).xpect_contains(&second);

	// the commit is transactional across the pair, and both halves persisted
	host.stored(SCHEMA)
		.await
		.to_schema()
		.unwrap()
		.get_field_schema(&FieldPath::new(["is_really_difficult"]))
		.unwrap()
		.into_owned()
		.xpect_eq(ValueSchema::Bool(default()));
	// an optional field needs no backfill, so the rows are untouched
	host.stored(TODOS).await.value.xpect_eq(rows);
}
