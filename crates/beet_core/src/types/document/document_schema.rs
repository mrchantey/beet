use crate::prelude::*;
use bevy::reflect::Typed;

/// An optional schema describing the shape of a [`Document`](super::Document).
///
/// Paired with a `Document` on the same entity, it lets field writes be
/// type-checked. A field's schema is derived by walking the document schema
/// with the field's [`FieldPath`]. When absent, writes are untyped.
///
/// Wraps a [`FieldSchema`] so a document can either inline its [`ValueSchema`]
/// or reference a registered Rust type by path.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DocumentSchema(pub FieldSchema);

impl DocumentSchema {
	/// Build a schema describing the whole document from a Rust type.
	///
	/// The schema is inlined immediately via [`ValueSchema::of`], so no type
	/// registry is needed to type-check field writes.
	pub fn of<T: Typed>() -> Self {
		Self(FieldSchema::Inline(ValueSchema::of::<T>()))
	}

	/// Build a schema from an inline [`ValueSchema`].
	pub fn inline(schema: ValueSchema) -> Self {
		Self(FieldSchema::Inline(schema))
	}

	/// Build a schema referencing a registered Rust type by its path.
	///
	/// Type-checks against this variant are skipped unless the schema is first
	/// resolved against a registry, since the path alone is not enough.
	pub fn type_path<T: TypePath>() -> Self { Self(FieldSchema::of::<T>()) }

	/// Validate `value` against this schema, naming `subject` on failure.
	///
	/// The read backstop: documents can diverge from their schema outside the
	/// editor, so a required field missing at read is a hard error naming the
	/// field and the document, never a silently substituted default.
	/// [`OnMissing`] policies deliberately play no part here; they belong to
	/// [`SchemaCommit`].
	///
	/// A schema that cannot yet be resolved (a schema document still in flight)
	/// validates as a wildcard, so an arriving schema tightens the invariant
	/// rather than a missing one inventing failures.
	pub async fn assert_valid(
		&self,
		resolver: SchemaResolver<'_>,
		subject: &str,
		value: &mut Value,
	) -> Result {
		self.0
			.resolve(resolver)?
			.assert_valid_in(resolver, subject, value)
			.await
	}

	/// Assert the field at `path` accepts a value of type `T`.
	///
	/// Mirrors `FieldSchema::assert_eq_ty` on the token side. Passes silently
	/// when the schema does not resolve (no registry in hand, or a schema
	/// document still arriving) or when either side is [`ValueSchema::Any`].
	pub fn assert_field_type<T: Typed>(
		&self,
		resolver: SchemaResolver<'_>,
		path: &[FieldSegment],
	) -> Result {
		let Ok(schema) = self.0.resolve(resolver) else {
			return Ok(());
		};
		schema
			.get_field_schema_in(resolver, path)?
			.assert_matches(&ValueSchema::of::<T>(), path)
	}

	/// Assert the field at `path` is a list whose items accept type `T`.
	///
	/// List length and uniqueness constraints are ignored; only the item type
	/// is checked. Used by the list CRUD actions.
	pub fn assert_list_item_type<T: Typed>(
		&self,
		resolver: SchemaResolver<'_>,
		path: &[FieldSegment],
	) -> Result {
		let Ok(schema) = self.0.resolve(resolver) else {
			return Ok(());
		};
		match schema.get_field_schema_in(resolver, path)? {
			ValueSchema::Any => Ok(()),
			ValueSchema::List(list) => {
				list.item.assert_matches(&ValueSchema::of::<T>(), path)
			}
			other => bevybail!(
				"Field Schema Mismatch at `{}`\nExpected: list\nReceived: `{other:?}`",
				FieldPath::from(path)
			),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct CountDoc {
		count: i64,
		todos: Vec<String>,
	}

	#[crate::test]
	fn assert_field_type_matches() {
		let schema = DocumentSchema::of::<CountDoc>();
		let resolver = SchemaResolver::default();
		schema
			.assert_field_type::<i64>(resolver, &[FieldSegment::key("count")])
			.unwrap();
		schema
			.assert_field_type::<String>(resolver, &[FieldSegment::key(
				"count",
			)])
			.is_err()
			.xpect_true();
	}

	#[crate::test]
	fn assert_list_item_type_matches() {
		let schema = DocumentSchema::of::<CountDoc>();
		let resolver = SchemaResolver::default();
		schema
			.assert_list_item_type::<String>(resolver, &[FieldSegment::key(
				"todos",
			)])
			.unwrap();
		schema
			.assert_list_item_type::<i64>(resolver, &[FieldSegment::key(
				"todos",
			)])
			.is_err()
			.xpect_true();
		// non-list field
		schema
			.assert_list_item_type::<i64>(resolver, &[FieldSegment::key(
				"count",
			)])
			.is_err()
			.xpect_true();
	}

	/// The read backstop names both the document and the field it is missing.
	#[crate::test]
	async fn missing_required_field_is_an_error() {
		DocumentSchema::of::<CountDoc>()
			.assert_valid(
				SchemaResolver::default(),
				"data.json",
				&mut value!({ "todos": [] }),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("data.json")
			.xpect_contains("count");
	}

	/// A field of an authored schema named by reference is reachable, so a
	/// typed write into a composed document is checked rather than skipped.
	#[crate::test]
	fn a_referenced_schema_is_reachable() {
		let mut registry = SchemaRegistry::default();
		registry.insert("TodoItem", ValueSchema::of::<CountDoc>());
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let schema =
			DocumentSchema::inline(ValueSchema::Reference("TodoItem".into()));
		schema
			.assert_field_type::<i64>(resolver, &[FieldSegment::key("count")])
			.unwrap();
		schema
			.assert_field_type::<String>(resolver, &[FieldSegment::key(
				"count",
			)])
			.is_err()
			.xpect_true();
	}

	#[crate::test]
	fn any_schema_passes() {
		let schema = DocumentSchema::inline(ValueSchema::Any);
		let resolver = SchemaResolver::default();
		schema
			.assert_field_type::<String>(resolver, &[FieldSegment::key(
				"whatever",
			)])
			.unwrap();
		schema
			.assert_list_item_type::<i64>(resolver, &[FieldSegment::key(
				"whatever",
			)])
			.unwrap();
	}
}
