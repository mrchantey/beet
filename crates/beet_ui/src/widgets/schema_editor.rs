//! [`SchemaEditor`]: editing a schema with the machinery that renders the data
//! it describes.
//!
//! The keystone closure, whole. A schema is a value, so the schema of a schema
//! (the meta-schema) describes it, so the editor **is** a [`DynamicForm`] over
//! [`ValueSchema::meta`]. There is no schema-editing vocabulary left in this
//! file — no kinds to offer, no field-edit record, no apply — because the form
//! that edits a todo item is the form that edits the todo item's schema.
//!
//! Nothing here writes a schema document field by field. A schema edit changes
//! what existing data must satisfy, so it is a **transaction**: the editor edits
//! a [`DraftOf`] the schema document, forked once and sticky, and the form syncs
//! into that draft continuously and freely. The schema document is written only
//! by the commit, and only through [`TypedDocument::commit_schema`], which
//! evolves the schema document and the data document together or neither. What
//! it refuses lands in the editor's error line, which is where item 21's "add a
//! default" conversation happens, with the drafted schema still in place to be
//! fixed — or discarded with Revert.
use crate::prelude::*;
use beet_core::prelude::*;

/// An editor for a *schema* document: a generated form over the meta-schema,
/// bound to a sticky draft of that document, and a transactional commit behind
/// its submit button.
///
/// Name the schema document with a [`DocRef`], exactly as any foreign document
/// is named; the **data** document the schema describes is the one the editor
/// is mounted in, and a commit evolves the pair together, backfilling every
/// existing row (item 21) or refusing and touching neither.
///
/// Every schema edit is expressible, because the form is generated from the
/// meta-schema rather than from a curated set of operations: adding a field is
/// the `fields` list's add button, retyping one is its `schema` variant select,
/// removing one is its remove button, and a backfill for existing rows is the
/// `on_missing` field, whose `Default` payload is typed by the sibling `schema`
/// it names ([`SchemaRef::AtField`]).
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
		<div>
			// the draft is an ordinary document, so the generated controls sync
			// into it continuously without ever touching the schema. It carries
			// a `Document` from the start, so a control built before the fork
			// binds the draft rather than falling through to the data document.
			<div {(SchemaDraft, Document::default())}>
				<DynamicForm schema={ValueSchema::meta()} {SchemaEditForm}>
					<Button>"Apply"</Button>
					<Button action=true variant={ButtonVariant::Text} {RevertButton}>
						"Revert"
					</Button>
				</DynamicForm>
			</div>
			// the `.error-text` rule directly rather than `ErrorText`, whose
			// message is a static prop: this one is written by the commit
			<span {Classes::new([classes::ERROR_TEXT])}>
				{(Value::str(""), ErrorLine)}
			</span>
		</div>
	}
}

/// Marks a [`SchemaEditor`]'s draft document, the fork of the schema document
/// the editor's [`DocRef`] names.
///
/// The relation itself is derived rather than authored, exactly as [`FieldOf`]
/// is: the editor is authored with one `DocRef` and the draft's origin follows
/// from it, so an author never names the same document twice.
#[derive(Component)]
pub(super) struct SchemaDraft;

/// Marks the editor's error line, the whole report of a commit.
#[derive(Component)]
struct ErrorLine;

/// Marks the editor's revert button.
#[derive(Component)]
#[component(on_add = hook_ext::observe(revert_draft_on_click))]
struct RevertButton;

/// Marks the `<form>` a [`SchemaEditor`] commits through, wiring the commit to
/// its [`Submit`].
///
/// The commit boundary of item 29: the drafted schema applies as one
/// transaction, never as a stream of per-field syncs into the schema. The
/// submission's gathered *values* are unread — the draft is what is committed,
/// and it is already whole — so this rides `Submit` for the boundary, not for
/// the payload.
#[derive(Component)]
#[component(on_add = hook_ext::observe(commit_schema_edit))]
struct SchemaEditForm;

/// System: give each editor's draft the document it forks from, the one its
/// [`DocRef`] names.
pub(super) fn link_schema_drafts(
	drafts: Populated<Entity, (With<SchemaDraft>, Without<DraftOf>)>,
	editors: AncestorQuery<&DocRef>,
	mut commands: Commands,
) {
	for draft in drafts.iter() {
		// a `DocRef` arriving late (the spread lands with the editor's own
		// entity) leaves the draft unlinked and tries again next frame
		if let Ok(doc_ref) = editors.get_exclusive(draft) {
			commands.entity(draft).insert(DraftOf(doc_ref.document()));
		}
	}
}

/// Observer: the revert button discards the draft and forks the schema document
/// again, the explicit half of a sticky draft.
fn revert_draft_on_click(
	ev: On<PointerUp>,
	buttons: Query<(), With<RevertButton>>,
	drafts: AncestorQuery<Entity, With<SchemaDraft>>,
	mut commands: Commands,
) -> Result {
	// the event bubbles; act only at the button itself
	let target = ev.event_target();
	if !buttons.contains(target) {
		return OK;
	}
	let draft = drafts.get_exclusive(target)?;
	commands.trigger(RevertDraft { draft });
	OK
}

/// Apply the drafted schema to the schema document the editor names and the data
/// document it is mounted in, reporting the outcome in the editor's error line.
///
/// A refused commit is the editor's ordinary conversation, not a raised error:
/// "this field needs a value for existing rows" is what the author reads, fixes
/// in the still-drafted schema, and resubmits. A miswired editor reports there
/// too, naming what is missing.
fn commit_schema_edit(
	ev: On<Submit>,
	drafts: AncestorQuery<Entity, With<SchemaDraft>>,
	editors: AncestorQuery<&DocRef>,
	hosts: AncestorQuery<Entity, With<Document>>,
	children: Query<&Children>,
	error_lines: Query<(), With<ErrorLine>>,
	commands: AsyncCommands,
) -> Result {
	// resolution is structural and synchronous: the draft is the document the
	// form sits in, the schema document is the one the editor's `DocRef` above it
	// names, and the data document is the one the editor is mounted in.
	let draft = drafts.get_exclusive(ev.form)?;
	let editor = editors.get_entity(draft)?;
	let targets = editors.get_exclusive(draft).and_then(|doc_ref| {
		(doc_ref.document(), hosts.get_exclusive(draft)?).xok()
	});
	let error_line = children
		.iter_descendants(editor)
		.find(|entity| error_lines.contains(*entity));
	// off a task, never inline: item 20 makes `OnMissing::Computed` an async js
	// script, so evolving data is genuinely async, and blocking the world on it
	// would freeze every other binding and deadlock the thread the script needs.
	commands.run(async move |world| {
		let outcome = match targets {
			Ok((schema_doc, data_doc)) => {
				commit(&world, draft, schema_doc, data_doc).await
			}
			Err(err) => Err(err),
		};
		report(&world, error_line, outcome).await
	});
	OK
}

/// The commit itself, across two exclusive world hops: read the pair and the
/// draft out, evolve them, then write them back and publish the new schema so
/// every subtree generated from it rebuilds.
///
/// The registry is *cloned* into the task rather than borrowed, because no world
/// borrow may be held across the evolution's awaits. The gap between the hops is
/// the one window in which a concurrent write to either document would be lost;
/// multi-writer arbitration is the workstream item 15 parks, and until it lands
/// the editor is the only writer of a schema.
async fn commit(
	world: &AsyncWorld,
	draft: Entity,
	schema_doc: Entity,
	data_doc: Entity,
) -> Result {
	let (registry, mut declaration, mut data, next) = world
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
				// a schema document is a `ValueSchema` stored as data, so its
				// own arm is the meta-schema by definition
				TypedDocument::new(
					ValueSchema::type_ref::<ValueSchema>(),
					document_value(world, schema_doc, "`DocRef`")?,
				),
				TypedDocument::new(
					data_schema,
					document_value(world, data_doc, "host")?,
				),
				// the drafted schema, read as the schema it is a value of
				document_value(world, draft, "draft")?
					.into_serde::<ValueSchema>()?,
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
			*world.get_mut::<Document>(schema_doc).ok_or_else(|| {
				bevyhow!("the schema document was despawned")
			})? = Document::new(declaration.value);
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

/// Write the commit's outcome into the error line: the message when it was
/// refused, else nothing, leaving the draft in place either way.
///
/// A spent draft is deliberately *not* reset: it now holds exactly what was
/// committed, which is where the next edit starts from.
async fn report(
	world: &AsyncWorld,
	error_line: Option<Entity>,
	outcome: Result,
) -> Result {
	let message = outcome.err().map(|err| err.to_string()).unwrap_or_default();
	let Some(error_line) = error_line else {
		bevybail!("the schema editor has no error line to report {message}");
	};
	world
		.entity(error_line)
		.with(move |mut entity: EntityWorldMut| -> Result {
			entity
				.get_mut::<Value>()
				.ok_or_else(|| bevyhow!("the error line holds no value"))?
				.set_if_neq(Value::str(message));
			OK
		})
		.await?
}

/// A document's value, or a message naming the role of the entity that has none.
///
/// The loud half of item 27: a `DocRef` names an entity declared to *be* a
/// document, so one that answers none is an error rather than a fallback.
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
	use super::super::test_ext;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `{ label: String }`, the row schema the editor edits.
	fn todo_schema(fields: Vec<NamedFieldSchema>) -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("TodoItem".into()),
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

	/// The editor's draft, ie the schema being edited.
	fn draft(world: &mut World) -> Entity {
		world
			.query_once::<(Entity, &DraftOf)>()
			.into_iter()
			.map(|(entity, _)| entity)
			.next()
			.unwrap()
	}

	/// Draft `schema` and press Apply, the whole input of a commit.
	fn apply(world: &mut World, schema: &ValueSchema) {
		let draft = draft(world);
		world.entity_mut(draft).get_mut::<Document>().unwrap().0 =
			Value::from_serde(schema).unwrap();
		settle(world);
		let button = test_ext::submit_button(world);
		world.entity_mut(button).trigger(PointerUp::new(button));
		settle(world);
	}

	fn document(world: &mut World, entity: Entity) -> Value {
		world.entity(entity).get::<Document>().unwrap().0.clone()
	}

	/// The [`ValueSchema`] a schema document holds, read back the way any
	/// consumer does.
	fn schema_of(world: &mut World, entity: Entity) -> ValueSchema {
		TypedDocument::new(
			ValueSchema::type_ref::<ValueSchema>(),
			document(world, entity),
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

	/// Item 3's acceptance loop: commit a schema with an extra bool field, and
	/// the schema document declares it, every existing row is backfilled, and
	/// the view generated from that schema grows the column.
	#[beet_core::test]
	fn adding_a_field_evolves_the_schema_the_data_and_the_view() {
		let (mut world, schema_doc, data_doc) = app();
		test_ext::render_world(&mut world, data_doc)
			.xnot()
			.xpect_contains("is_really_difficult");

		apply(
			&mut world,
			&with_difficulty(Some(OnMissing::Default(value!(false)))),
		);

		// the schema document declares the field
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["is_really_difficult"]))
			.unwrap()
			.xpect_eq(ValueSchema::Bool(default()));
		// every existing row was backfilled
		document(&mut world, data_doc).xpect_eq(
			value!({ "items": [{ "label": "buy milk", "is_really_difficult": false }] }),
		);
		// ...and the table generated from that schema grew the column
		test_ext::render_world(&mut world, data_doc)
			.xpect_contains("<th>is_really_difficult</th>")
			.xpect_contains("buy milk");
	}

	/// The same loop through the *generated controls*, which is the closure
	/// itself: the add button of the meta-schema's own `fields` list appends a
	/// field, its key is typed into a generated text control, and Apply commits
	/// what they drafted.
	#[beet_core::test]
	fn adding_a_field_through_the_generated_controls() {
		let (mut world, schema_doc, data_doc) = app();
		let add = test_ext::collection_add(&mut world, "Struct.fields");
		test_ext::click_world(&mut world, add);
		settle(&mut world);

		// the appended row is the item schema's zero, so it arrives as a real
		// `NamedFieldSchema` rather than a null
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

		let button = test_ext::submit_button(&mut world);
		test_ext::click_world(&mut world, button);
		settle(&mut world);

		error(&mut world).xpect_eq("");
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["note"]))
			.unwrap()
			.xpect_eq(ValueSchema::String(default()));
		test_ext::render_world(&mut world, data_doc)
			.xpect_contains("<th>note</th>");
	}

	/// A required field with nothing for existing rows is refused, and the
	/// refusal names the field: item 21's conversation, in the error line.
	/// Both documents are left exactly as they were, and so is the draft.
	#[beet_core::test]
	fn a_required_field_without_a_value_is_refused() {
		let (mut world, schema_doc, data_doc) = app();
		let (schema_before, data_before) = (
			document(&mut world, schema_doc),
			document(&mut world, data_doc),
		);

		let refused = with_difficulty(None);
		apply(&mut world, &refused);

		error(&mut world).xpect_contains("is_really_difficult");
		document(&mut world, schema_doc).xpect_eq(schema_before);
		document(&mut world, data_doc).xpect_eq(data_before);
		// the drafted schema stays put, to be fixed and resubmitted
		let draft = draft(&mut world);
		document(&mut world, draft)
			.xpect_eq(Value::from_serde(&refused).unwrap());
	}

	/// Retyping is the edit item 21 most often refuses: existing values must
	/// validate under the new schema, or the commit names the conversion it
	/// would need and touches neither document.
	#[beet_core::test]
	fn a_retype_is_accepted_only_where_the_values_survive() {
		let (mut world, schema_doc, data_doc) = app();
		let before = (
			document(&mut world, schema_doc),
			document(&mut world, data_doc),
		);
		apply(
			&mut world,
			&todo_schema(vec![NamedFieldSchema::new(
				"label",
				ValueSchema::U64(default()),
			)]),
		);
		error(&mut world)
			.xpect_contains("label")
			.xpect_contains("computed conversion");
		(
			document(&mut world, schema_doc),
			document(&mut world, data_doc),
		)
			.xpect_eq(before);

		// an optional field no existing row carries has nothing to invalidate
		apply(
			&mut world,
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
			.xpect_eq(ValueSchema::U64(default()));
	}

	/// A refusal is not sticky: the next commit that succeeds clears the error
	/// line.
	#[beet_core::test]
	fn a_later_success_clears_the_refusal() {
		let (mut world, _, _) = app();
		apply(&mut world, &with_difficulty(None));
		error(&mut world).xpect_contains("is_really_difficult");

		apply(
			&mut world,
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
		apply(&mut world, &todo_schema(vec![]));
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["label"]))
			.is_err()
			.xpect_true();
		document(&mut world, data_doc).xpect_eq(value!({ "items": [{}] }));
	}

	/// The editor is a form over the meta-schema, so the schema document's own
	/// fields are the controls: the keystone closure, rendered.
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
	}

	/// The draft is forked from the schema document and is **sticky**: editing
	/// it leaves the schema document alone, which is what makes the commit a
	/// commit rather than a stream of field syncs.
	#[beet_core::test]
	fn the_draft_is_a_sticky_fork_of_the_schema_document() {
		let (mut world, schema_doc, _) = app();
		let draft = draft(&mut world);
		world
			.entity(draft)
			.get::<DraftOf>()
			.unwrap()
			.origin()
			.xpect_eq(schema_doc);
		document(&mut world, draft).xpect_eq(document(&mut world, schema_doc));

		let before = document(&mut world, schema_doc);
		world.entity_mut(draft).get_mut::<Document>().unwrap().0 =
			Value::from_serde(&todo_schema(vec![])).unwrap();
		settle(&mut world);
		document(&mut world, schema_doc).xpect_eq(before);
	}

	/// Revert is the way back out of a draft, and it is a button rather than an
	/// automatic reset, because a refused commit must leave the edit in place.
	#[beet_core::test]
	fn reverting_discards_the_draft() {
		let (mut world, schema_doc, _) = app();
		let draft = draft(&mut world);
		world.entity_mut(draft).get_mut::<Document>().unwrap().0 =
			Value::from_serde(&todo_schema(vec![])).unwrap();
		settle(&mut world);

		let revert = world
			.query_once::<(Entity, &super::RevertButton)>()
			.into_iter()
			.map(|(entity, _)| entity)
			.next()
			.unwrap();
		test_ext::click_world(&mut world, revert);
		document(&mut world, draft).xpect_eq(document(&mut world, schema_doc));
	}

	/// The same loop driven by the terminal: press the `fields` list's own add
	/// button, type the new field's key with real keys, press Apply, and the view
	/// generated from that schema grows the column.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn typing_an_edit_and_applying_it_adds_the_column() {
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

		let button = test_ext::submit_button(app.world_mut());
		test_ext::click(&mut app, button);
		settle(&mut app);

		test_ext::render_world(app.world_mut(), data_doc)
			.xpect_contains("<th>note</th>");
	}
}
