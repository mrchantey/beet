//! [`DataDocument`]: the self-describing on-disk form of an editable document.
use crate::prelude::*;
use bevy_reflect::TypeRegistry;

/// A runtime-editable document as it is stored: `{ "schema": .., "value": .. }`.
///
/// A `.bsx` file is the authored *original* state of an application; anything
/// the running application may rewrite is a data document, so it carries its own
/// schema rather than depending on the binary that opens it to know the shape.
/// The three ways it can name that schema are the three [`FieldSchema`] arms.
///
/// Landing it on an entity ([`bundle`](Self::bundle)) produces exactly the
/// [`Document`] / [`DocumentSchema`] pair the binding layer already syncs.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DataDocument {
	/// How to resolve the schema the value must satisfy.
	pub schema: FieldSchema,
	/// The document's value.
	pub value: Value,
}

impl DataDocument {
	/// Declare a data document from a schema and its value.
	pub fn new(schema: FieldSchema, value: Value) -> Self {
		Self { schema, value }
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
		registry: &TypeRegistry,
		subject: &str,
		json: &str,
	) -> Result<Self> {
		let mut document: Self = serde_json::from_str(json)?;
		document.assert_valid(subject, registry).await?;
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
		subject: &str,
		registry: &TypeRegistry,
	) -> Result {
		DocumentSchema(self.schema.clone())
			.assert_valid(subject, &mut self.value, registry)
			.await
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;
	use bevy_reflect::TypeRegistry;

	/// Read `json` as `"todos.json"`, the subject every test here names.
	async fn read(registry: &TypeRegistry, json: &str) -> DataDocument {
		DataDocument::read(registry, "todos.json", json)
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

	#[crate::test]
	async fn round_trips_byte_identically() {
		let registry = TypeRegistry::default();
		let json = read(&registry, todo_json()).await.to_json().unwrap();
		// a reopen of the re-serialized bytes produces the same bytes again
		read(&registry, &json)
			.await
			.to_json()
			.unwrap()
			.xpect_eq(json);
	}

	/// The read backstop: a document whose value lost a required field fails at
	/// read, naming both the document and the field.
	#[crate::test]
	async fn read_rejects_a_diverged_document() {
		DataDocument::read(
			&TypeRegistry::default(),
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
		let registry = TypeRegistry::default();
		FieldSchema::document("schema.json")
			.resolve(&registry)
			.unwrap()
			.xpect_eq(ValueSchema::Any);
		FieldSchema::inline(ValueSchema::Bool(default()))
			.resolve(&registry)
			.unwrap()
			.xpect_eq(ValueSchema::Bool(default()));
		FieldSchema::TypePath("nope::Nope".into())
			.resolve(&registry)
			.is_err()
			.xpect_true();
	}

	#[crate::test]
	async fn value_lands_as_a_document_pair() {
		let mut world = DocumentPlugin::world();
		let document = read(&TypeRegistry::default(), todo_json()).await;
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
}
