//! [`SchemaEditor`]: editing a schema with the machinery that renders the data
//! it describes.
//!
//! The keystone closure made concrete. A schema is a value, so the schema of a
//! schema (the meta-schema) describes it, so the widgets that lay out a todo
//! item lay out its *schema*: the current fields are a [`DynamicView`] over the
//! meta-schema's own `NamedFieldSchema`, and the pending edit is a
//! [`DynamicForm`] over [`FieldEdit`]'s.
//!
//! Nothing here writes a schema document field by field. A schema edit changes
//! what existing data must satisfy, so it is a **transaction**: the drafted
//! edit lives in the editor's own document, and the form's `Submit` applies it
//! through [`TypedDocument::commit_schema`], which evolves the schema document
//! and the data document together or neither. What it refuses lands in the
//! editor's error line, which is where item 21's "add a default" conversation
//! happens.
use crate::prelude::*;
use beet_core::prelude::*;

/// The draft field holding the last commit's failure, empty when it succeeded.
const ERROR_FIELD: &str = "error";

/// The path to a struct schema's field list within a schema document, ie the
/// `fields` of the meta-schema's `Struct` variant payload.
fn fields_path() -> FieldPath { FieldPath::new(["Struct", "fields"]) }

/// A fresh draft: an empty edit and no error, the state the editor opens in and
/// returns to once a commit is spent.
fn fresh_draft() -> Result<Value> {
	let mut draft = Value::from_serde(FieldEdit::default())?;
	draft.insert(ERROR_FIELD, Value::str(""))?;
	draft.xok()
}

/// An editor for a *schema* document: the fields it declares as a table, one
/// drafted field edit as a generated form, and a transactional commit behind
/// the form's submit button.
///
/// Name the schema document with a [`DocRef`], exactly as any foreign document
/// is named; the **data** document the schema describes is the one the editor
/// is mounted in, and a commit evolves the pair together, backfilling every
/// existing row (item 21) or refusing and touching neither.
///
/// One edit at a time, keyed by field name: an unknown name adds the field, a
/// known one retypes it, and `remove` drops it. A field the commit would leave
/// required-but-absent needs a value for existing rows, and says so.
///
/// ```rsx
/// <div {data.bundle()}>
///   <Fragment bx:ref="schema" {schema.bundle()}/>
///   <SchemaEditor {DocRef($schema)}/>
/// </div>
/// ```
#[template]
pub fn SchemaEditor() -> Result<impl Bundle> {
	// what a schema document holds at `Struct.fields`, asked of the meta-schema
	// rather than restated here
	let meta = ValueSchema::meta();
	let fields = meta.get_field_schema(&fields_path()).map(Clone::clone)?;
	let draft = fresh_draft()?;

	rsx! {
		<div>
			// the schema document's own fields, read-only by construction: a
			// bound text node carries no element, so nothing can type into one
			// and bypass the commit
			<DynamicView schema={fields} field={FieldRef::new(fields_path())}/>
			// the draft is the editor's own document, so the generated controls
			// sync into it continuously without ever touching the schema
			<div {Document::new(draft)}>
				<DynamicForm schema={FieldEdit::schema()} {SchemaEditForm}>
					<Button>"Apply"</Button>
				</DynamicForm>
				// the `.error-text` rule directly rather than `ErrorText`,
				// whose message is a static prop: this one is bound, so a
				// commit's report reflows into it like any other value
				<span {Classes::new([classes::ERROR_TEXT])}>
					{FieldRef::new(ERROR_FIELD)}
				</span>
			</div>
		</div>
	}
	.xok()
}

/// The value kinds a [`SchemaEditor`] gives a field: one per [`ValueSchema`]
/// variant [`DynamicForm`] dispatches to a control, since a field the form
/// cannot edit is a field the editor should not create.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Reflect, Serialize, Deserialize,
)]
pub enum SchemaKind {
	/// [`ValueSchema::String`], edited by a text field.
	#[default]
	String,
	/// [`ValueSchema::Bool`], edited by a checkbox.
	Bool,
	/// [`ValueSchema::I64`], edited by a number field.
	I64,
	/// [`ValueSchema::U64`], edited by a number field.
	U64,
	/// [`ValueSchema::F64`], edited by a number field.
	F64,
}

impl SchemaKind {
	/// Every kind, in the order the generated `<select>` offers them.
	pub const ALL: [Self; 5] =
		[Self::String, Self::Bool, Self::I64, Self::U64, Self::F64];

	/// The schema a field of this kind takes, its variant's default.
	pub fn schema(&self) -> ValueSchema {
		match self {
			Self::String => ValueSchema::String(default()),
			Self::Bool => ValueSchema::Bool(default()),
			Self::I64 => ValueSchema::I64(default()),
			Self::U64 => ValueSchema::U64(default()),
			Self::F64 => ValueSchema::F64(default()),
		}
	}

	/// Read `text` as a value of this kind, the backfill a commit assigns to
	/// every row that has none.
	///
	/// Kind-directed rather than optimistic: `"3"` is a string for a `String`
	/// field and a number for a numeric one, so the parsed value satisfies the
	/// schema that asked for it.
	pub fn parse(&self, text: &str) -> Result<Value> {
		let text = text.trim();
		match self {
			Self::String => Some(Value::str(text)),
			Self::Bool => text.parse().ok().map(Value::Bool),
			Self::I64 => text.parse().ok().map(Value::Int),
			Self::U64 => text.parse().ok().map(Value::Uint),
			Self::F64 => text.parse().ok().map(Value::Float),
		}
		.ok_or_else(|| bevyhow!("`{text}` is not a {self:?} value"))
	}
}

/// One drafted field edit: the value a [`SchemaEditor`]'s generated form edits,
/// and the whole input of a commit.
///
/// Its own schema is what [`DynamicForm`] walks, so the editor's controls are
/// generated exactly as the form over the data is.
#[derive(Debug, Default, Clone, PartialEq, Reflect, Serialize, Deserialize)]
pub struct FieldEdit {
	/// The field to add, retype or remove, by key.
	pub key: String,
	/// The kind the field takes.
	pub kind: SchemaKind,
	/// Whether the field must be present, which is what makes a value for
	/// existing rows mandatory rather than optional.
	pub required: bool,
	/// The value existing rows are backfilled with; empty declares no
	/// resolution, which a required field is rejected for.
	pub default: String,
	/// Remove the named field rather than adding it.
	pub remove: bool,
}

impl FieldEdit {
	/// The schema the editor's form is generated from: this type's own,
	/// relabelled so each control reads as what it does rather than as a struct
	/// field name.
	pub fn schema() -> ValueSchema {
		let mut schema = ValueSchema::of::<Self>();
		if let ValueSchema::Struct(fields) = &mut schema {
			for field in &mut fields.fields {
				field.label = match field.key.as_str() {
					"key" => Some("field name".into()),
					"kind" => Some("type".into()),
					"default" => Some("value for existing rows".into()),
					"remove" => Some("remove instead".into()),
					_ => continue,
				};
			}
		}
		schema
	}

	/// Apply this edit to the struct schema a schema document holds.
	///
	/// Only the schema is changed here; whether the *data* survives the change
	/// is [`SchemaCommit`]'s judgement, made against every existing row.
	fn apply(&self, schema: &mut ValueSchema) -> Result {
		let key = self.key.trim();
		if key.is_empty() {
			bevybail!("name the field to edit");
		}
		let ValueSchema::Struct(schema) = schema else {
			bevybail!(
				"a `{}` schema has no named fields to edit; a schema editor \
				edits a struct schema",
				schema.variant_name()
			);
		};
		let existing = schema.fields.iter().position(|field| field.key == key);
		if self.remove {
			let Some(index) = existing else {
				bevybail!("no field `{key}` to remove");
			};
			schema.fields.remove(index);
			return OK;
		}
		let mut field = NamedFieldSchema::new(key, self.kind.schema());
		if !self.required {
			field = field.optional();
		}
		if !self.default.trim().is_empty() {
			field = field.with_on_missing(OnMissing::Default(
				self.kind.parse(&self.default)?,
			));
		}
		match existing {
			// a retype keeps the field where it was, so the columns hold still
			Some(index) => schema.fields[index] = field,
			None => schema.fields.push(field),
		}
		OK
	}
}

/// Marks the `<form>` a [`SchemaEditor`] commits through, wiring the commit to
/// its [`Submit`].
///
/// The commit boundary of item 29: everything the form gathers applies as one
/// transaction, never as a stream of per-field syncs into the schema.
#[derive(Component)]
#[component(on_add = hook_ext::observe(commit_schema_edit))]
struct SchemaEditForm;

/// Apply the drafted edit to the schema document the editor names and the data
/// document it is mounted in, reporting the outcome in the editor's error line.
///
/// A refused commit is the editor's ordinary conversation, not a raised error:
/// "this field needs a value for existing rows" is what the author reads, types
/// and resubmits. A miswired editor reports there too, naming what is missing.
fn commit_schema_edit(
	ev: On<Submit>,
	editors: AncestorQuery<&DocRef>,
	hosts: AncestorQuery<Entity, With<Document>>,
	mut documents: Query<&mut Document>,
	schemas: Query<&DocumentSchema>,
	mut registry: ResMut<SchemaRegistry>,
	mut commands: Commands,
) -> Result {
	let form = ev.form;
	// the draft is the form's own document, the nearest one above it; the
	// schema document is named by the editor's `DocRef` above that, and the data
	// document is the one the editor as a whole is mounted in.
	let draft = hosts.get_exclusive(form)?;
	let outcome = editors.get_exclusive(draft).and_then(|doc_ref| {
		let data_doc = hosts.get_exclusive(draft)?;
		commit(
			&ev.values,
			doc_ref.document(),
			data_doc,
			&mut documents,
			&schemas,
			&mut registry,
			&mut commands,
		)
	});
	let mut draft_document = documents.get_mut(draft)?;
	match outcome {
		// the edit is spent: a fresh draft, so the next one starts clean and
		// the error line clears with it
		Ok(()) => draft_document.0 = fresh_draft()?,
		Err(err) => {
			draft_document
				.0
				.insert(ERROR_FIELD, Value::str(err.to_string()))?;
		}
	}
	OK
}

/// The commit itself: read the pair, apply the edit to the schema, evolve the
/// data under it, and publish the result so every subtree generated from that
/// schema rebuilds.
fn commit(
	values: &Value,
	schema_doc: Entity,
	data_doc: Entity,
	documents: &mut Query<&mut Document>,
	schemas: &Query<&DocumentSchema>,
	registry: &mut SchemaRegistry,
	commands: &mut Commands,
) -> Result {
	let Ok(data_schema) = schemas.get(data_doc).map(|schema| schema.0.clone())
	else {
		bevybail!(
			"the document a schema editor is mounted in declares no schema, \
			so a schema edit has no data to evolve"
		);
	};
	// the declaration's own arm is the meta-schema by definition: a schema
	// document is a `ValueSchema` stored as data.
	let mut declaration = TypedDocument::new(
		FieldSchema::TypePath(ValueSchema::type_path().into()),
		documents.get(schema_doc)?.0.clone(),
	);
	let mut data = TypedDocument::new(
		data_schema.clone(),
		documents.get(data_doc)?.0.clone(),
	);
	let mut next = declaration.to_schema()?;
	values.clone().into_serde::<FieldEdit>()?.apply(&mut next)?;

	// one poll: every resolution but `OnMissing::Computed` (which is the js
	// runtime seam, and panics) settles without an executor.
	async_ext::try_block_on(data.commit_schema(
		SchemaResolver::default().with_schemas(registry),
		"the data document",
		&mut declaration,
		next.clone(),
	))??;

	documents.get_mut(schema_doc)?.0 = declaration.value;
	documents.get_mut(data_doc)?.0 = data.value;
	if data.schema != data_schema {
		commands
			.entity(data_doc)
			.insert(DocumentSchema(data.schema.clone()));
	}
	// the same publication `commit_schema` evolved the data against, now made
	// real, so a `Reference` to this schema resolves to what was committed
	match &data.schema {
		FieldSchema::Document(path) => {
			registry.insert_located(path.clone(), next)
		}
		_ => {
			if let Some(name) = next.name().cloned() {
				registry.insert(name, next);
			}
		}
	}
	OK
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
							item: Box::new(ValueSchema::Reference("TodoItem".into())),
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
						item: Box::new(ValueSchema::Reference(
							"TodoItem".into(),
						)),
						..default()
					}),
				)],
			})),
		));
		(schema_doc, data_doc)
	}

	/// Run the frames a commit needs: the submit, the document syncs it dirties,
	/// the registry-driven rebuild, and that rebuild's own first sync.
	fn settle(world: &mut World) {
		for _ in 0..4 {
			world.update_local();
		}
	}

	/// Fill the editor's drafted edit and press its submit button.
	fn apply(world: &mut World, edit: FieldEdit) {
		let draft = world
			.query_once::<(Entity, &Document)>()
			.into_iter()
			.find(|(_, doc)| doc.0.get("key").is_some())
			.map(|(entity, _)| entity)
			.unwrap();
		world.entity_mut(draft).get_mut::<Document>().unwrap().0 =
			Value::from_serde(edit).unwrap();
		settle(world);

		let button = world
			.query_once::<(Entity, &Element)>()
			.into_iter()
			.find(|(_, element)| element.tag() == "button")
			.map(|(entity, _)| entity)
			.unwrap();
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
			FieldSchema::TypePath(ValueSchema::type_path().into()),
			document(world, entity),
		)
		.to_schema()
		.unwrap()
	}

	/// The editor's own error line, the commit's whole report.
	fn error(world: &mut World) -> String {
		world
			.query_once::<(&ResolvedFieldPath, &Value)>()
			.into_iter()
			.find(|(resolved, _)| resolved.field_path.to_string() == "error")
			.map(|(_, value)| value.to_string())
			.unwrap()
	}

	/// Item 3's acceptance loop through the widgets: add a bool field to the
	/// schema, and the schema document declares it, every existing row is
	/// backfilled, and the view generated from that schema grows the column.
	#[beet_core::test]
	fn adding_a_field_evolves_the_schema_the_data_and_the_view() {
		let (mut world, schema_doc, data_doc) = app();
		test_ext::render_world(&mut world, data_doc)
			.xnot()
			.xpect_contains("is_really_difficult");

		apply(&mut world, FieldEdit {
			key: "is_really_difficult".into(),
			kind: SchemaKind::Bool,
			required: true,
			default: "false".into(),
			remove: false,
		});

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

	/// The same loop driven by the terminal: type the field name into the
	/// editor's own generated control, click Apply, and the view generated from
	/// that schema grows the column. An optional field needs no value for
	/// existing rows, so the default draft commits as typed.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn typing_an_edit_and_applying_it_adds_the_column() {
		let mut app = test_ext::form_app();
		let (_, data_doc) = spawn_app(app.world_mut());
		test_ext::settle(&mut app);

		// the editor's `key` control, the one element bound to that draft path
		let key_input = app
			.world_mut()
			.query_once::<(Entity, &ResolvedFieldPath, &Element)>()
			.into_iter()
			.find(|(_, resolved, _)| resolved.field_path.to_string() == "key")
			.map(|(entity, ..)| entity)
			.unwrap();
		let window = app.world_mut().spawn_empty().id();
		app.world_mut()
			.entity_mut(key_input)
			.insert((Focus, RenderSurface(window)));
		test_ext::type_text(&mut app, window, "note");

		let button = test_ext::element(&mut app, "button");
		test_ext::click(&mut app, button);
		test_ext::settle(&mut app);

		test_ext::render_world(app.world_mut(), data_doc)
			.xpect_contains("<th>note</th>");
	}

	/// A required field with nothing for existing rows is refused, and the
	/// refusal names the field: item 21's conversation, in the error line.
	/// Both documents are left exactly as they were.
	#[beet_core::test]
	fn a_required_field_without_a_value_is_refused() {
		let (mut world, schema_doc, data_doc) = app();
		let (schema_before, data_before) = (
			document(&mut world, schema_doc),
			document(&mut world, data_doc),
		);

		apply(&mut world, FieldEdit {
			key: "is_really_difficult".into(),
			kind: SchemaKind::Bool,
			required: true,
			default: String::new(),
			remove: false,
		});

		error(&mut world).xpect_contains("is_really_difficult");
		document(&mut world, schema_doc).xpect_eq(schema_before);
		document(&mut world, data_doc).xpect_eq(data_before);
	}

	/// Retyping is the third operation the editor offers, and the one item 21
	/// most often refuses: existing values must validate under the new kind, or
	/// the commit names the conversion it would need and touches neither
	/// document. A field no row carries retypes freely.
	#[beet_core::test]
	fn a_retype_is_accepted_only_where_the_values_survive() {
		let (mut world, schema_doc, data_doc) = app();
		let before = (
			document(&mut world, schema_doc),
			document(&mut world, data_doc),
		);
		apply(&mut world, FieldEdit {
			key: "label".into(),
			kind: SchemaKind::U64,
			required: true,
			..default()
		});
		error(&mut world)
			.xpect_contains("label")
			.xpect_contains("computed conversion");
		(
			document(&mut world, schema_doc),
			document(&mut world, data_doc),
		)
			.xpect_eq(before);

		// an optional field no existing row carries has nothing to invalidate
		for kind in [SchemaKind::String, SchemaKind::U64] {
			apply(&mut world, FieldEdit {
				key: "count".into(),
				kind,
				..default()
			});
			error(&mut world).xpect_eq("");
		}
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["count"]))
			.unwrap()
			.xpect_eq(ValueSchema::U64(default()));
	}

	/// A refusal is not sticky: the next commit that succeeds clears the error
	/// line with the draft it spends.
	#[beet_core::test]
	fn a_later_success_clears_the_refusal() {
		let (mut world, _, _) = app();
		apply(&mut world, FieldEdit {
			key: "is_really_difficult".into(),
			kind: SchemaKind::Bool,
			required: true,
			..default()
		});
		error(&mut world).xpect_contains("is_really_difficult");

		apply(&mut world, FieldEdit {
			key: "is_really_difficult".into(),
			kind: SchemaKind::Bool,
			required: true,
			default: "false".into(),
			remove: false,
		});
		error(&mut world).xpect_eq("");
	}

	/// Removing a field is the same transaction in reverse, and an unknown one
	/// is refused rather than silently doing nothing.
	#[beet_core::test]
	fn removing_a_field_is_the_same_commit() {
		let (mut world, schema_doc, _) = app();
		apply(&mut world, FieldEdit {
			key: "nope".into(),
			remove: true,
			..default()
		});
		error(&mut world).xpect_contains("nope");

		apply(&mut world, FieldEdit {
			key: "label".into(),
			remove: true,
			..default()
		});
		schema_of(&mut world, schema_doc)
			.get_field_schema(&FieldPath::new(["label"]))
			.is_err()
			.xpect_true();
	}

	/// The editor lists the fields the schema document declares, generated from
	/// the meta-schema rather than authored: the keystone closure, rendered.
	#[beet_core::test]
	fn the_field_list_is_generated_from_the_meta_schema() {
		let (mut world, _, data_doc) = app();
		test_ext::render_world(&mut world, data_doc)
			// the meta-schema's own `NamedFieldSchema` columns
			.xpect_contains("<th>key</th>")
			.xpect_contains("<th>required</th>")
			.xpect_contains("<th>schema</th>")
			// ...over the schema document's one field
			.xpect_contains("label");
	}

	/// Every offered kind has a control: the editor never creates a field the
	/// form it generates cannot edit.
	#[beet_core::test]
	fn every_kind_dispatches_to_a_control() {
		for kind in SchemaKind::ALL {
			let mut world = world_ext::ui_world();
			let schema = kind.schema();
			world
				.spawn_template(rsx! {
					<DynamicForm schema={schema} field={FieldRef::new("field")}/>
				})
				.unwrap();
			world.update_local();
			world.query_once::<&UneditableField>().len().xpect_eq(0);
		}
	}

	/// The kinds are exactly the `<select>`'s options, so a kind added to the
	/// enum but not to `ALL` is caught rather than silently unofferable.
	#[beet_core::test]
	fn every_kind_is_offered() {
		let ValueSchema::Enum(kinds) = ValueSchema::of::<SchemaKind>() else {
			panic!("SchemaKind is a unit enum");
		};
		kinds.variants.len().xpect_eq(SchemaKind::ALL.len());
	}

	/// A value for existing rows is read as the kind that asked for it, so `"3"`
	/// backfills a string field with text and a numeric one with a number.
	#[beet_core::test]
	fn a_backfill_is_parsed_as_its_own_kind() {
		SchemaKind::String
			.parse("3")
			.unwrap()
			.xpect_eq(Value::str("3"));
		SchemaKind::U64.parse("3").unwrap().xpect_eq(Value::Uint(3));
		SchemaKind::Bool
			.parse("yes")
			.unwrap_err()
			.to_string()
			.xpect_contains("Bool");
	}
}
