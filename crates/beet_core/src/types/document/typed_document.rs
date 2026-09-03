//! [`TypedDocument`]: the self-describing on-disk form of an editable document.
use crate::prelude::*;

/// A runtime-editable document as it is stored: `{ "schema": .., "value": .. }`.
///
/// A `.bsx` file is the authored *original* state of an application; anything
/// the running application may rewrite is a typed document, so it carries its
/// own schema rather than depending on the binary that opens it to know the
/// shape.
/// The three ways it can name that schema are the three [`FieldSchema`] arms.
///
/// A **schema** document is one of these too, holding a [`ValueSchema`] as its
/// value and naming the meta-schema as its schema
/// ([`schema_document`](Self::schema_document)). That closure is what makes the
/// schema editable by the same machinery as the data it describes.
///
/// Landing it on an entity ([`bundle`](Self::bundle)) produces exactly the
/// [`Document`] / [`DocumentSchema`] pair the binding layer already syncs.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypedDocument {
	/// How to resolve the schema the value must satisfy.
	pub schema: FieldSchema,
	/// The document's value.
	pub value: Value,
}

impl TypedDocument {
	/// Declare a data document from a schema and its value.
	pub fn new(schema: FieldSchema, value: Value) -> Self {
		Self { schema, value }
	}

	/// A schema document: `schema` stored as data, described by the
	/// meta-schema.
	///
	/// The keystone closure of schema-as-data. A schema document is an ordinary
	/// document, so it loads, binds, validates and saves through exactly the
	/// machinery that carries the data it describes.
	#[cfg(feature = "serde")]
	pub fn schema_document(schema: &ValueSchema) -> Result<Self> {
		Self::new(
			FieldSchema::TypePath(ValueSchema::type_path().into()),
			Value::from_serde(schema)?,
		)
		.xok()
	}

	/// The [`ValueSchema`] this schema document holds.
	#[cfg(feature = "serde")]
	pub fn to_schema(&self) -> Result<ValueSchema> {
		self.value.clone().into_serde()
	}

	/// The components a data document lands on an entity as.
	pub fn bundle(self) -> impl Bundle {
		(Document::new(self.value), DocumentSchema(self.schema))
	}

	/// Read a data document from JSON, validating it against its own schema.
	///
	/// Validating here rather than leaving it to the caller is the read
	/// backstop: a document that diverged outside the editor must fail loudly
	/// at the moment it is read, naming `subject` and the offending field, so
	/// no reading path can quietly accept data its schema rejects.
	#[cfg(feature = "json")]
	pub async fn read(
		resolver: SchemaResolver<'_>,
		subject: &str,
		json: &str,
	) -> Result<Self> {
		let mut document: Self = serde_json::from_str(json)?;
		document.assert_valid(resolver, subject).await?;
		document.xok()
	}

	/// Serialize to JSON.
	///
	/// Byte-deterministic: [`Map`] emits its keys sorted, so an unchanged
	/// document reopens and re-saves byte-identically.
	#[cfg(feature = "json")]
	pub fn to_json(&self) -> Result<String> {
		serde_json::to_string_pretty(self).map_err(Into::into)
	}

	/// Validate the value against its schema, naming `subject` on failure.
	///
	/// [`read`](Self::read) runs this, so this is for revalidating a document
	/// the running application has since edited.
	pub async fn assert_valid(
		&mut self,
		resolver: SchemaResolver<'_>,
		subject: &str,
	) -> Result {
		DocumentSchema(self.schema.clone())
			.assert_valid(resolver, subject, &mut self.value)
			.await
	}

	/// Evolve this data document under a new `schema` for `declaration`, the
	/// schema document it names.
	///
	/// Item 21's commit at document scope: the new schema and every backfill it
	/// resolves apply together or not at all, so a rejected commit leaves both
	/// documents byte-identical to what they were. The new schema is validated
	/// against the meta-schema before any data is touched, and a document whose
	/// schema is a Rust type is refused: a type changes by rebuilding the
	/// binary, not by an editor.
	///
	/// The data document's own declaration is not rewritten, because it names
	/// *where* its schema lives and that does not move when the schema changes.
	/// The one exception is a document that inlines the very schema being
	/// committed, which has no separate declaration to point at.
	#[cfg(feature = "serde")]
	pub async fn commit_schema(
		&mut self,
		resolver: SchemaResolver<'_>,
		subject: &str,
		declaration: &mut Self,
		schema: ValueSchema,
	) -> Result {
		if let FieldSchema::TypePath(path) = &self.schema {
			bevybail!(
				"{subject} is described by the Rust type `{path}`, which an \
				editor cannot evolve; author its schema as a document first"
			);
		}
		// the schema document's own value first: a schema that is not a value
		// of the meta-schema never reaches the data.
		let mut declared = Value::from_serde(&schema)?;
		ValueSchema::meta()
			.assert_valid_in(resolver, subject, &mut declared)
			.await?;

		// the registry as it will be once the declaration lands, so the data is
		// evolved against the schema the commit is about to publish rather than
		// the one it is replacing.
		let mut overlay = resolver.registry().cloned().unwrap_or_default();
		match &self.schema {
			FieldSchema::Document(path) => {
				overlay.insert_located(path.clone(), schema.clone())
			}
			// anything else reaches the declaration by name, if it has one
			_ => {
				if let Some(name) = schema.name() {
					overlay.insert(name.clone(), schema.clone());
				}
			}
		}
		let resolver = resolver.with_schemas(&overlay);

		let mut next = self.clone();
		// a document inlining exactly what the declaration held is its own
		// declaration, so the schema moves with it; one that merely composes the
		// declaration by reference keeps the composition it was authored with.
		if let FieldSchema::Inline(inline) = &mut next.schema
			&& declaration.to_schema().ok().as_ref() == Some(inline)
		{
			*inline = schema;
		}
		SchemaCommit::new(next.schema.resolve(resolver)?)
			.apply_in(resolver, &mut next.value)
			.await?;

		declaration.value = declared;
		*self = next;
		OK
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	/// Read `json` as `"todos.json"`, the subject every test here names.
	async fn read(resolver: SchemaResolver<'_>, json: &str) -> TypedDocument {
		TypedDocument::read(resolver, "todos.json", json)
			.await
			.unwrap()
	}

	fn todo_json() -> &'static str {
		r#"{
			"schema": { "Inline": { "Struct": {
				"name": "TodoItem",
				"allow_additional": false,
				"fields": [{
					"key": "label",
					"required": true,
					"label": null,
					"description": null,
					"on_missing": null,
					"schema": { "String": { "sensitive": false, "constraints": [] } }
				}]
			} } },
			"value": { "label": "buy milk" }
		}"#
	}

	/// `{ label: String }` authored as data, the schema the todo app edits.
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

	#[crate::test]
	async fn round_trips_byte_identically() {
		let resolver = SchemaResolver::default();
		let json = read(resolver, todo_json()).await.to_json().unwrap();
		// a reopen of the re-serialized bytes produces the same bytes again
		read(resolver, &json)
			.await
			.to_json()
			.unwrap()
			.xpect_eq(json);
	}

	/// The read backstop: a document whose value lost a required field fails at
	/// read, naming both the document and the field.
	#[crate::test]
	async fn read_rejects_a_diverged_document() {
		TypedDocument::read(
			SchemaResolver::default(),
			"todos.json",
			&todo_json().replace(r#"{ "label": "buy milk" }"#, "{}"),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("todos.json")
		.xpect_contains("label");
	}

	/// The three arms are the whole vocabulary: inline, by document, by type
	/// path. A document reference defers to a wildcard until it arrives.
	#[crate::test]
	fn schema_arms() {
		let resolver = SchemaResolver::default();
		FieldSchema::document("schema.json")
			.resolve(resolver)
			.unwrap()
			.xpect_eq(ValueSchema::Any);
		FieldSchema::inline(ValueSchema::Bool(default()))
			.resolve(resolver)
			.unwrap()
			.xpect_eq(ValueSchema::Bool(default()));
		FieldSchema::TypePath("nope::Nope".into())
			.resolve(resolver)
			.is_err()
			.xpect_true();
	}

	#[crate::test]
	async fn value_lands_as_a_document_pair() {
		let mut world = DocumentPlugin::world();
		let document = read(SchemaResolver::default(), todo_json()).await;
		let entity = world.spawn(document.bundle()).id();
		world
			.entity(entity)
			.get::<Document>()
			.unwrap()
			.get_field_ref(&[FieldSegment::key("label")])
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("buy milk");
	}

	/// A schema document round trips through its own value, and validates
	/// against the meta-schema like any other document against its schema.
	#[crate::test]
	async fn a_schema_is_a_document() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let schema = todo_schema(vec![label()]);
		let mut document = TypedDocument::schema_document(&schema).unwrap();
		document
			.assert_valid(resolver, "schema.json")
			.await
			.unwrap();
		document.to_schema().unwrap().xpect_eq(schema);
		// and it survives the json round trip the editor saves through
		read(resolver, &document.to_json().unwrap())
			.await
			.xpect_eq(document);
	}

	/// The acceptance loop of item 3 at the data layer: add a bool field to the
	/// schema document, and the data document's rows grow the column.
	#[crate::test]
	async fn a_schema_edit_evolves_both_documents() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let mut declaration =
			TypedDocument::schema_document(&todo_schema(vec![label()]))
				.unwrap();
		let mut data = TypedDocument::new(
			FieldSchema::document("schema.json"),
			value!({ "label": "buy milk" }),
		);

		let difficult = NamedFieldSchema::new(
			"is_really_difficult",
			ValueSchema::Bool(default()),
		)
		.with_on_missing(OnMissing::Default(value!(false)));
		data.commit_schema(
			resolver,
			"todos.json",
			&mut declaration,
			todo_schema(vec![label(), difficult]),
		)
		.await
		.unwrap();

		data.value.xpect_eq(
			value!({ "label": "buy milk", "is_really_difficult": false }),
		);
		// the declaration moved with it, and still names its schema document
		declaration
			.to_schema()
			.unwrap()
			.get_field_schema(&[FieldSegment::key("is_really_difficult")])
			.unwrap()
			.xpect_eq(ValueSchema::Bool(default()));
		data.schema.xpect_eq(FieldSchema::document("schema.json"));
	}

	/// The todo app's own shape: the schema document holds the *row* schema and
	/// the data document only composes it by reference, so a schema edit
	/// backfills every row while the data document's declaration stays put.
	#[crate::test]
	async fn a_row_schema_edit_backfills_every_row() {
		let mut registry = SchemaRegistry::default();
		registry.insert("TodoItem", todo_schema(vec![label()]));
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let mut declaration =
			TypedDocument::schema_document(&todo_schema(vec![label()]))
				.unwrap();
		let rows = ValueSchema::List(ListSchema {
			item: Box::new(ValueSchema::Reference("TodoItem".into())),
			min_items: None,
			max_items: None,
			unique: false,
		});
		let mut data = TypedDocument::new(
			FieldSchema::inline(rows.clone()),
			value!([{ "label": "a" }, { "label": "b" }]),
		);

		data.commit_schema(
			resolver,
			"todos.json",
			&mut declaration,
			todo_schema(vec![
				label(),
				NamedFieldSchema::new(
					"is_really_difficult",
					ValueSchema::Bool(default()),
				)
				.with_on_missing(OnMissing::Default(value!(false))),
			]),
		)
		.await
		.unwrap();

		data.value.xpect_eq(value!([
			{ "label": "a", "is_really_difficult": false },
			{ "label": "b", "is_really_difficult": false },
		]));
		// the data document still composes its rows exactly as authored
		data.schema.xpect_eq(FieldSchema::inline(rows));
	}

	/// A commit with no resolution for what it breaks leaves *both* documents
	/// untouched: the transaction spans the pair.
	#[crate::test]
	async fn a_rejected_commit_touches_neither_document() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let before = todo_schema(vec![label()]);
		let mut declaration = TypedDocument::schema_document(&before).unwrap();
		let mut data = TypedDocument::new(
			FieldSchema::inline(before.clone()),
			value!({ "label": "buy milk" }),
		);
		let (declaration_before, data_before) =
			(declaration.clone(), data.clone());

		data.commit_schema(
			resolver,
			"todos.json",
			&mut declaration,
			todo_schema(vec![
				label(),
				NamedFieldSchema::new(
					"is_really_difficult",
					ValueSchema::Bool(default()),
				),
			]),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("is_really_difficult");

		declaration.xpect_eq(declaration_before);
		data.xpect_eq(data_before);
	}

	/// A document described by a Rust type is not runtime-editable: evolving it
	/// means rebuilding the binary, and saying so beats a silent no-op.
	#[crate::test]
	async fn a_type_backed_document_refuses_to_evolve() {
		let mut declaration =
			TypedDocument::schema_document(&ValueSchema::Any).unwrap();
		TypedDocument::new(FieldSchema::of::<i64>(), value!(7))
			.commit_schema(
				SchemaResolver::default(),
				"count.json",
				&mut declaration,
				ValueSchema::of::<i64>(),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("i64");
	}
}
