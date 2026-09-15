//! [`SchemaEditor`]: editing a schema with the machinery that renders the data
//! it describes.
//!
//! The keystone closure, whole. A schema is a value, so the schema of a schema
//! (the meta-schema) describes it, so the editor **is** a [`DynamicForm`] over
//! [`ValueSchema::meta`]. There is no schema-editing vocabulary left in this
//! file — no kinds to offer, no field-edit record, no draft, no apply —
//! because the form that edits a todo item is the form that edits the todo
//! item's schema, and it edits the schema document exactly as that form edits
//! the item: every control lands in the document under its own
//! [`WritePolicy`].
//!
//! What survives is the **transaction**. A schema edit changes what existing
//! data must satisfy, so every change of the schema document is committed
//! against the data document it describes through
//! [`TypedDocument::commit_schema`], which evolves the pair together or
//! neither. A refused commit reverts the schema document to what was last
//! committed, snapping the control that asked back, and lands its reason in
//! the editor's error line, which is where item 21's "add a default"
//! conversation happens.
use crate::prelude::*;
use beet_core::prelude::*;

/// An editor for a *schema* document: a generated form over the meta-schema,
/// bound to that document, with the transactional commit behind every edit.
///
/// Name the schema document with a [`DocRef`], exactly as any foreign document
/// is named; the **data** document the schema describes is the one the editor
/// is mounted in, and each edit evolves the pair together, backfilling every
/// existing row (item 21) or refusing and touching neither.
///
/// Every schema edit is expressible, because the form is generated from the
/// meta-schema rather than from a curated set of operations: adding a field is
/// the `fields` list's add button, retyping one is its `schema` variant select,
/// removing one is its remove button, and a backfill for existing rows is the
/// `on_missing` field, whose `Default` payload is typed by the sibling `schema`
/// it names ([`SchemaRef::AtField`]). A field's key lands on blur (the
/// meta-schema says so), since half a key is a rename of every row's column;
/// everything else lands as typed or picked.
///
/// ```rsx
/// <div {data.bundle()}>
///   <Fragment bx:ref="schema" {schema.bundle()}/>
///   <SchemaEditor {DocRef($schema)}/>
/// </div>
/// ```
#[template]
pub fn SchemaEditor() -> impl Bundle {
	rsx! {
		<div {SchemaEditorRoot::default()}>
			// the form binds the schema document through the editor's `DocRef`,
			// so the generated controls edit it directly, per their policies
			<DynamicForm schema={ValueSchema::meta()}/>
			// the `.error-text` rule directly rather than `ErrorText`, whose
			// message is a static prop: this one is written by the commit
			<span {Classes::new([classes::ERROR_TEXT])}>
				{(Value::str(""), ErrorLine)}
			</span>
		</div>
	}
}

/// Edit mode: a [`SchemaEditor`] behind a closed disclosure, so an app ships the
/// ability to change its own shape without wearing it.
///
/// Item 11's opt-in, as a component rather than a feature: an app that wants no
/// schema editing simply does not author this, and one that does gets the
/// toggle, the editor and the whole commit path with one tag. The `DocRef`
/// naming the schema document rides this tag, since the editor resolves it by
/// ancestor walk.
///
/// The disclosure is a plain `<details>`, closed by default: the terminal
/// collapses it exactly as the browser does, so edit mode costs no widget of its
/// own and stays true on every surface.
///
/// ```rsx
/// <div {data.bundle()}>
///   <Fragment bx:ref="schema" {schema.bundle()}/>
///   <ToggleSchemaEditor {DocRef($schema)}/>
/// </div>
/// ```
#[template]
pub fn ToggleSchemaEditor(
	/// The disclosure's label.
	#[prop(default = "Edit schema".to_string())]
	label: String,
) -> impl Bundle {
	rsx! {
		<details>
			<summary>{label}</summary>
			<SchemaEditor/>
		</details>
	}
}

/// Marks a [`SchemaEditor`]'s root, remembering what its schema document last
/// committed.
///
/// The document itself is what the form edits, so the editor holds no copy of
/// it: only the value the last commit accepted, which is what a change is
/// measured against and what a refused one reverts to.
#[derive(Debug, Default, Component)]
pub(in crate::widgets) struct SchemaEditorRoot {
	/// The schema document's value as last committed, `None` until the
	/// document has answered.
	committed: Option<Value>,
	/// Whether a commit is in flight. A change landing meanwhile is committed
	/// after it, against what it left committed.
	in_flight: bool,
}

/// Marks the editor's error line, the whole report of a commit.
#[derive(Component)]
pub(in crate::widgets) struct ErrorLine;

/// System: commit each changed schema document against the data document its
/// editor is mounted in, one commit in flight per editor.
///
/// Resolution is structural: the schema document is the one the editor's
/// [`DocRef`] names, and the data document is the nearest document above the
/// editor (item 87's three-document resolution, less the draft). A schema
/// document that has not arrived yet leaves the editor waiting rather than
/// reporting, since a store read lands frames after the tree is built.
pub(in crate::widgets) fn commit_schema_edits(
	mut editors: Query<(Entity, &mut SchemaEditorRoot)>,
	doc_refs: AncestorQuery<&DocRef>,
	documents: Query<Ref<Document>>,
	resolver: DocumentResolver,
	children: Query<&Children>,
	error_lines: Query<(), With<ErrorLine>>,
	commands: AsyncCommands,
) -> Result {
	for (root, mut state) in editors.iter_mut() {
		let Ok(editor) = doc_refs.get_entity(root) else {
			continue;
		};
		let schema_doc = doc_refs.get(root)?.document();
		let Ok(document) = documents.get(schema_doc) else {
			continue;
		};
		// the first sighting is the baseline, not an edit
		let Some(committed) = &state.committed else {
			state.committed = Some(document.0.clone());
			continue;
		};
		if state.in_flight
			|| !(document.is_changed() || state.is_changed())
			|| document.0 == *committed
		{
			continue;
		}
		let (base, next) = (committed.clone(), document.0.clone());
		state.in_flight = true;
		// the data document is the one the editor sits in, resolved from
		// above the editor's own `DocRef` so the walk does not end at the
		// schema document it names
		let data_doc = resolver.entity_above(editor, &DocumentPath::Ancestor);
		let error_line = children
			.iter_descendants(root)
			.find(|entity| error_lines.contains(*entity));
		// off a task, never inline: item 20 makes `OnMissing::Computed` an
		// async js script, so evolving data is genuinely async, and blocking
		// the world on it would freeze every other binding and deadlock the
		// thread the script needs.
		commands.run(async move |world| {
			let outcome = commit(&world, data_doc, &base, &next).await;
			finish(&world, root, schema_doc, error_line, base, next, outcome)
				.await
		});
	}
	OK
}

/// The commit itself, across two exclusive world hops: read the data document
/// out, evolve it under `next` against `base`, then write it back and publish
/// the new schema so every subtree generated from it rebuilds. The schema
/// document already holds `next`, so only the data moves here.
///
/// The registry is *cloned* into the task rather than borrowed, because no world
/// borrow may be held across the evolution's awaits. The gap between the hops is
/// the one window in which a concurrent write to the data document would be
/// lost; multi-writer arbitration is the workstream item 15 parks, and until it
/// lands the editor is the only writer of a schema.
async fn commit(
	world: &AsyncWorld,
	data_doc: Entity,
	base: &Value,
	next: &Value,
) -> Result {
	// a value the form is still shaping may not be a schema yet, which is
	// a refusal like any other
	let next = next.clone().into_serde::<ValueSchema>()?;
	// a schema document is a `ValueSchema` stored as data, so its own arm is
	// the meta-schema by definition
	let mut declaration = TypedDocument::new(
		ValueSchema::type_ref::<ValueSchema>(),
		base.clone(),
	);
	let (registry, mut data) = world
		.with(move |world: &mut World| {
			let Some(data_schema) = world
				.get::<DocumentSchema>(data_doc)
				.map(|schema| schema.0.clone())
			else {
				bevybail!(
					"the document a schema editor is mounted in declares no \
					schema, so a schema edit has no data to evolve"
				);
			};
			(
				world.resource::<SchemaRegistry>().clone(),
				TypedDocument::new(
					data_schema,
					document_value(world, data_doc, "host")?,
				),
			)
				.xok()
		})
		.await?;

	data.commit_schema(
		SchemaResolver::default().with_schemas(&registry),
		"the data document",
		&mut declaration,
		next.clone(),
	)
	.await?;

	world
		.with(move |world: &mut World| {
			*world
				.get_mut::<Document>(data_doc)
				.ok_or_else(|| bevyhow!("the data document was despawned"))? =
				Document::new(data.value);
			// the data document's own declaration moves only when the commit
			// moved it, ie when it inlined exactly what the schema document held
			if world
				.get::<DocumentSchema>(data_doc)
				.map(|schema| &schema.0)
				!= Some(&data.schema)
			{
				world
					.entity_mut(data_doc)
					.insert(DocumentSchema(data.schema.clone()));
			}
			// the publication `commit_schema` evolved the data against, now made
			// real, so a `Reference` to this schema resolves to what was committed
			let mut registry = world.resource_mut::<SchemaRegistry>();
			match &data.schema {
				ValueSchema::Ref(SchemaRef::Document(path)) => {
					registry.insert_located(path.clone(), next)
				}
				_ => {
					if let Some(name) = next.name().cloned() {
						registry.insert(name, next);
					}
				}
			}
			OK
		})
		.await
}

/// Settle a commit: an accepted `next` becomes what the editor measures the
/// next change against, a refused one reverts the schema document to `base`,
/// and the error line reads the refusal or nothing.
async fn finish(
	world: &AsyncWorld,
	root: Entity,
	schema_doc: Entity,
	error_line: Option<Entity>,
	base: Value,
	next: Value,
	outcome: Result,
) -> Result {
	let (committed, message) = match outcome {
		Ok(()) => (next, String::new()),
		Err(err) => (base, commit_message(err)),
	};
	let Some(error_line) = error_line else {
		bevybail!("the schema editor has no error line to report {message}");
	};
	world
		.with(move |world: &mut World| -> Result {
			if !message.is_empty()
				&& let Some(mut document) =
					world.get_mut::<Document>(schema_doc)
			{
				document.set_if_neq(Document::new(committed.clone()));
			}
			if let Some(mut state) = world.get_mut::<SchemaEditorRoot>(root) {
				state.committed = Some(committed);
				state.in_flight = false;
			}
			world
				.get_mut::<Value>(error_line)
				.ok_or_else(|| bevyhow!("the error line holds no value"))?
				.set_if_neq(Value::str(message));
			OK
		})
		.await
}

/// The reader-facing half of a refused commit: the error itself, without the
/// trace that follows it.
///
/// [`BevyError`]'s `Display` writes the message and then its backtrace, which
/// is a developer's artifact — a person told to add a default does not need the
/// crate path and line number of the check that refused them.
fn commit_message(error: BevyError) -> String {
	error
		.to_string()
		.lines()
		.next()
		.unwrap_or_default()
		.to_string()
}

/// A document's value, or a message naming the role of the entity that has none.
///
/// The loud half of item 27: the editor is mounted in an entity declared to
/// *be* a document, so one that answers none is an error rather than a
/// fallback.
fn document_value(world: &World, entity: Entity, role: &str) -> Result<Value> {
	world
		.get::<Document>(entity)
		.map(|document| document.0.clone())
		.ok_or_else(|| {
			bevyhow!("the schema editor\'s {role} entity holds no document")
		})
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::widgets::schema_ui::test_ext;
	use beet_core::prelude::*;

	/// `{ label: String }`, the row schema the editor edits.
	fn todo_schema(fields: Vec<NamedFieldSchema>) -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("TodoItem".into()),
			description: None,
			allow_additional: false,
			fields,
		})
	}

	fn label() -> NamedFieldSchema {
		NamedFieldSchema::new("label", ValueSchema::String(default()))
	}

	/// The todo app's shape, settled: a data document of rows composing the row
	/// schema by reference, a schema document holding that row schema, a view
	/// generated from it, and an editor pointed at it.
	fn app() -> (World, Entity, Entity) {
		let mut world = test_ext::form_world();
		let (schema_doc, data_doc) = spawn_app(&mut world);
		settle(&mut world);
		(world, schema_doc, data_doc)
	}

	/// [`app`]'s shape, built into `world` so the terminal-driven test can build
	/// the same one into a live [`App`].
	fn spawn_app(world: &mut World) -> (Entity, Entity) {
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert("TodoItem", todo_schema(vec![label()]));
		let schema_doc = world
			.spawn(
				TypedDocument::schema_document(&todo_schema(vec![label()]))
					.unwrap()
					.bundle(),
			)
			.id();
		let data_doc = world
			.spawn_template(rsx! {
				<div>
					<DynamicView
						schema={ValueSchema::List(ListSchema {
							item: Box::new(ValueSchema::reference("TodoItem")),
							..default()
						})}
						field={FieldRef::new("items")}
					/>
					<SchemaEditor {DocRef(schema_doc)}/>
				</div>
			})
			.unwrap()
			.id();
		world.entity_mut(data_doc).insert((
			Document::new(value!({ "items": [{ "label": "buy milk" }] })),
			DocumentSchema::inline(ValueSchema::Struct(StructSchema {
				name: None,
				description: None,
				allow_additional: false,
				fields: vec![NamedFieldSchema::new(
					"items",
					ValueSchema::List(ListSchema {
						item: Box::new(ValueSchema::Ref(SchemaRef::Name(
							"TodoItem".into(),
						))),
						..default()
					}),
				)],
			})),
		));
		(schema_doc, data_doc)
	}

	/// Run the frames a commit needs, plus the ones its regenerated controls
	/// take to arrive: a nested generation syncs, rebuilds and syncs again.
	fn settle(world: &mut World) {
		for _ in 0..8 {
			world.update_local();
		}
	}

	/// Write `schema` into the schema document from outside the form, the
	/// whole input of a commit however a control lands it.
	fn edit(world: &mut World, schema_doc: Entity, schema: &ValueSchema) {
		world
			.entity_mut(schema_doc)
			.get_mut::<Document>()
			.unwrap()
			.0 = Value::from_serde(schema).unwrap();
		settle(world);
	}

	/// The [`ValueSchema`] a schema document holds, read back the way any
	/// consumer does.
	fn schema_of(world: &mut World, entity: Entity) -> ValueSchema {
		TypedDocument::new(
			ValueSchema::type_ref::<ValueSchema>(),
			test_ext::document_of(world, entity),
		)
		.to_schema()
		.unwrap()
	}

	/// The editor's own error line, the commit's whole report.
	fn error(world: &mut World) -> String {
		world
			.query_once::<(&super::ErrorLine, &Value)>()
			.into_iter()
			.map(|(_, value)| value.to_string())
			.next()
			.unwrap()
	}

	/// `label` plus a required bool field with a value for existing rows, ie
	/// item 3's edit expressed as the schema it produces.
	fn with_difficulty(on_missing: Option<OnMissing>) -> ValueSchema {
		let mut field = NamedFieldSchema::new(
			"is_really_difficult",
			ValueSchema::Bool(default()),
		);
		field.on_missing = on_missing;
		todo_schema(vec![label(), field])
	}

	/// Item 3's acceptance loop: a schema with an extra bool field lands, and
	/// the schema document declares it, every existing row is backfilled, and
	/// the view generated from that schema grows the column.
	#[beet_core::test]
	fn adding_a_field_evolves_the_schema_the_data_and_the_view() {
		let (mut world, schema_doc, data_doc) = app();
		test_ext::render_world(&mut world, data_doc)
			.xnot()
			.xpect_contains("is_really_difficult");

		edit(
			&mut world,
			schema_doc,
			&with_difficulty(Some(OnMissing::Default(value!(false)))),
		);

		// the schema document declares the field
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["is_really_difficult"]))
			.unwrap()
			.into_owned()
			.xpect_eq(ValueSchema::Bool(default()));
		// every existing row was backfilled
		test_ext::document_of(&mut world, data_doc).xpect_eq(
			value!({ "items": [{ "label": "buy milk", "is_really_difficult": false }] }),
		);
		// ...and the table generated from that schema grew the column
		test_ext::render_world(&mut world, data_doc)
			.xpect_contains("<th>Is really difficult</th>")
			.xpect_contains("buy milk");
	}

	/// The same loop through the *generated controls*, which is the closure
	/// itself: the add button of the meta-schema's own `fields` list appends a
	/// field, its key is typed into a generated text control, its schema is
	/// chosen with the variant select, and each lands as its own commit with
	/// nothing to apply.
	#[beet_core::test]
	fn editing_through_the_generated_controls() {
		let (mut world, schema_doc, data_doc) = app();
		let add = test_ext::collection_add(&mut world, "Struct.fields");
		test_ext::click_world(&mut world, add);
		settle(&mut world);

		// the appended row is the item schema's zero, so it arrives as a real
		// `NamedFieldSchema` rather than a null, an optional `Any` field the
		// existing rows already satisfy
		let key = test_ext::bound(&mut world, "Struct.fields.[1].key");
		world
			.entity_mut(key)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str("note"));
		// ...and its schema is chosen with the variant select the meta-schema's
		// own enum generated
		let kind =
			test_ext::variant_select(&mut world, "Struct.fields.[1].schema");
		world
			.entity_mut(kind)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str("String"));
		settle(&mut world);

		error(&mut world).xpect_eq("");
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["note"]))
			.unwrap()
			.into_owned()
			.xpect_eq(ValueSchema::String(default()));
		test_ext::render_world(&mut world, data_doc)
			.xpect_contains("<th>Note</th>");
	}

	/// A required field with nothing for existing rows is refused, and the
	/// refusal names the field: item 21's conversation, in the error line.
	/// Both documents are left exactly as they were: the schema document is
	/// reverted to what was last committed.
	#[beet_core::test]
	fn a_required_field_without_a_value_is_refused() {
		let (mut world, schema_doc, data_doc) = app();
		let (schema_before, data_before) = (
			test_ext::document_of(&mut world, schema_doc),
			test_ext::document_of(&mut world, data_doc),
		);

		edit(&mut world, schema_doc, &with_difficulty(None));

		error(&mut world).xpect_contains("is_really_difficult");
		test_ext::document_of(&mut world, schema_doc).xpect_eq(schema_before);
		test_ext::document_of(&mut world, data_doc).xpect_eq(data_before);
	}

	/// A refusal snaps the control that asked back: the `required` box of a
	/// field no row has a value for reverts with the document, so the form
	/// never shows a schema the data does not satisfy.
	#[beet_core::test]
	fn a_refused_edit_snaps_the_control_back() {
		let (mut world, schema_doc, _) = app();
		let add = test_ext::collection_add(&mut world, "Struct.fields");
		test_ext::click_world(&mut world, add);
		settle(&mut world);
		let key = test_ext::bound(&mut world, "Struct.fields.[1].key");
		world
			.entity_mut(key)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::str("note"));
		settle(&mut world);
		error(&mut world).xpect_eq("");

		let required =
			test_ext::bound(&mut world, "Struct.fields.[1].required");
		world
			.entity_mut(required)
			.get_mut::<Value>()
			.unwrap()
			.set_if_neq(Value::Bool(true));
		settle(&mut world);
		error(&mut world).xpect_contains("note");
		world
			.entity(required)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Bool(false));
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["note"]))
			.is_ok()
			.xpect_true();
	}

	/// Retyping is the edit item 21 most often refuses: existing values must
	/// validate under the new schema, or the commit names the conversion it
	/// would need and touches neither document.
	#[beet_core::test]
	fn a_retype_is_accepted_only_where_the_values_survive() {
		let (mut world, schema_doc, data_doc) = app();
		let before = (
			test_ext::document_of(&mut world, schema_doc),
			test_ext::document_of(&mut world, data_doc),
		);
		edit(
			&mut world,
			schema_doc,
			&todo_schema(vec![NamedFieldSchema::new(
				"label",
				ValueSchema::U64(default()),
			)]),
		);
		error(&mut world)
			.xpect_contains("label")
			.xpect_contains("computed conversion");
		(
			test_ext::document_of(&mut world, schema_doc),
			test_ext::document_of(&mut world, data_doc),
		)
			.xpect_eq(before);

		// an optional field no existing row carries has nothing to invalidate
		edit(
			&mut world,
			schema_doc,
			&todo_schema(vec![
				label(),
				NamedFieldSchema::new("count", ValueSchema::U64(default()))
					.optional(),
			]),
		);
		error(&mut world).xpect_eq("");
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["count"]))
			.unwrap()
			.into_owned()
			.xpect_eq(ValueSchema::U64(default()));
	}

	/// A refusal is not sticky: the next commit that succeeds clears the error
	/// line.
	#[beet_core::test]
	fn a_later_success_clears_the_refusal() {
		let (mut world, schema_doc, _) = app();
		edit(&mut world, schema_doc, &with_difficulty(None));
		error(&mut world).xpect_contains("is_really_difficult");

		edit(
			&mut world,
			schema_doc,
			&with_difficulty(Some(OnMissing::Default(value!(false)))),
		);
		error(&mut world).xpect_eq("");
	}

	/// Removing a field is the same transaction in reverse: the field goes and
	/// so does its data, which is the reading `allow_additional: false` already
	/// had (item 89).
	#[beet_core::test]
	fn removing_a_field_is_the_same_commit() {
		let (mut world, schema_doc, data_doc) = app();
		edit(&mut world, schema_doc, &todo_schema(vec![]));
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["label"]))
			.is_err()
			.xpect_true();
		test_ext::document_of(&mut world, data_doc)
			.xpect_eq(value!({ "items": [{}] }));
	}

	/// The editor is a form over the meta-schema, so the schema document's own
	/// fields are the controls: the keystone closure, rendered. The key lands
	/// on blur and the rest as typed, as the meta-schema declares.
	#[beet_core::test]
	fn the_form_is_generated_from_the_meta_schema() {
		let (mut world, _, data_doc) = app();
		let html = test_ext::render_world(&mut world, data_doc);
		html.clone()
			// the schema is a value of the meta-schema's own enum...
			.xpect_contains("<option value=\"Struct\"")
			// ...whose `Struct` payload is a struct of named fields
			.xpect_contains("name=\"Struct.fields.[0].key\"")
			.xpect_contains("name=\"Struct.fields.[0].required\"")
			// ...each with its own schema, chosen by the same enum again
			.xpect_contains("name=\"Struct.fields.[0].schema\"");
		// and the field's key is the schema document's, not a placeholder
		html.xpect_contains("label");
		let key = test_ext::bound(&mut world, "Struct.fields.[0].key");
		world
			.entity(key)
			.get::<WritePolicy>()
			.copied()
			.xpect_eq(Some(WritePolicy::Blur));
		let required =
			test_ext::bound(&mut world, "Struct.fields.[0].required");
		world
			.entity(required)
			.get::<WritePolicy>()
			.copied()
			.xpect_eq(Some(WritePolicy::Input));
	}

	/// The same loop driven by the terminal: press the `fields` list's own add
	/// button, type the new field's key with real keys, and the column grows
	/// once the key control is left, not while it is typed into.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn typing_a_key_adds_the_column_on_blur() {
		let mut app = test_ext::form_app();
		let (_, data_doc) = spawn_app(app.world_mut());
		let settle = |app: &mut App| {
			for _ in 0..8 {
				app.update();
			}
		};
		settle(&mut app);

		let add = test_ext::collection_add(app.world_mut(), "Struct.fields");
		test_ext::click(&mut app, add);
		settle(&mut app);

		// the appended field's key control, typed into as the terminal does
		let key = test_ext::bound(app.world_mut(), "Struct.fields.[1].key");
		let window = app.world_mut().spawn_empty().id();
		app.world_mut()
			.entity_mut(key)
			.insert((Focus, RenderSurface(window)));
		test_ext::type_text(&mut app, window, "note");
		settle(&mut app);
		// held: the column is not there while the key is being typed
		test_ext::render_world(app.world_mut(), data_doc)
			.xnot()
			.xpect_contains("<th>Note</th>");

		// the blur lands it, and the commit grows the column
		app.world_mut().entity_mut(key).remove::<Focus>();
		settle(&mut app);
		test_ext::render_world(app.world_mut(), data_doc)
			.xpect_contains("<th>Note</th>");
	}
}
