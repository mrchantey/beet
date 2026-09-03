use crate::prelude::*;

/// Identifies the value type of a field or token, in exactly three ways.
///
/// This is how a self-describing data document names its schema: inline (the
/// [`ValueSchema`] in place), by document (another document in the same store,
/// resolved by **location**), or by Rust [`TypePath`] (a registered type,
/// resolved at runtime via the [`TypeRegistry`](bevy_reflect::TypeRegistry)).
///
/// Resolution by location sits *beside* the one [`SchemaRegistry`] namespace,
/// which resolves [`ValueSchema::Reference`] by **name**; it does not replace
/// it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FieldSchema {
	/// A Rust [`TypePath`], ie `bevy_color::color::Color`.
	TypePath(SmolStr),
	/// A schema defined inline, without a corresponding registered type.
	Inline(ValueSchema),
	/// Another document holding the schema, by default resolved in this
	/// document's own store (the `AncestorQuery<&BlobStore>` idiom).
	Document(SmolPath),
}

impl FieldSchema {
	/// Creates a schema from a Rust [`TypePath`].
	pub fn of<T: TypePath>() -> Self {
		Self::TypePath(SmolStr::new_static(T::type_path()))
	}

	/// Creates a schema from an inline [`ValueSchema`].
	pub fn inline(schema: ValueSchema) -> Self { Self::Inline(schema) }

	/// Creates a schema referencing another document by location.
	pub fn document(path: impl Into<SmolPath>) -> Self {
		Self::Document(path.into())
	}

	/// Resolve to a [`ValueSchema`] against `resolver`.
	///
	/// `Inline` variants are returned as-is. A `TypePath` resolves through the
	/// one [`SchemaRegistry`] namespace first and bevy's type registry second,
	/// so a hand-authored schema registered under a type path stands in for
	/// what reflection would derive. A `Document` resolves by **location**, in
	/// the by-location index a schema document read out of a store registers
	/// into; until it arrives the arm defers to [`ValueSchema::Any`] exactly as
	/// an unresolved [`ValueSchema::Reference`] does, and the read backstop
	/// ([`ValueSchema::assert_valid`]) tightens the invariant once it has.
	pub fn resolve(&self, resolver: SchemaResolver<'_>) -> Result<ValueSchema> {
		match self {
			Self::Inline(schema) => Ok(schema.clone()),
			Self::Document(path) => resolver
				.located(path)
				.cloned()
				.unwrap_or(ValueSchema::Any)
				.xok(),
			Self::TypePath(path) => resolver.type_schema(path),
		}
	}

	/// Returns the schema's identifying path, or `"inline"` for inline schemas.
	pub fn as_str(&self) -> &str {
		match self {
			Self::TypePath(path) => path.as_str(),
			Self::Document(path) => path.as_str(),
			Self::Inline(_) => "inline",
		}
	}

	/// Asserts that two schemas are equal.
	pub fn assert_eq(&self, other: &FieldSchema) -> Result<&Self> {
		if self == other {
			self.xok()
		} else {
			bevybail!(
				"Field Schema Mismatch\nExpected: `{other}`\nReceived: `{self}`"
			)
		}
	}

	/// Asserts that this schema's type path matches `T`.
	pub fn assert_eq_ty<T: TypePath>(&self) -> Result<&Self> {
		self.assert_eq(&Self::of::<T>())
	}
}

impl core::fmt::Display for FieldSchema {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::TypePath(s) => s.fmt(f),
			Self::Document(path) => path.fmt(f),
			Self::Inline(_) => write!(f, "inline"),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Reflect)]
	struct ResolveTarget {
		#[allow(dead_code)]
		count: u32,
	}

	#[crate::test]
	fn inline_resolves() {
		let inline = ValueSchema::Bool(BoolSchema::default());
		FieldSchema::inline(inline.clone())
			.resolve(SchemaResolver::default())
			.unwrap()
			.xpect_eq(inline);
	}

	#[crate::test]
	fn type_path_resolves_from_registry() {
		let mut types = bevy_reflect::TypeRegistry::default();
		types.register::<ResolveTarget>();
		let resolved = FieldSchema::of::<ResolveTarget>()
			.resolve(SchemaResolver::default().with_types(&types))
			.unwrap();
		matches!(resolved, ValueSchema::Struct(_)).xpect_true();
	}

	#[crate::test]
	fn type_path_missing_errors() {
		FieldSchema::of::<ResolveTarget>()
			.resolve(SchemaResolver::default())
			.is_err()
			.xpect_true();
	}

	/// Resolution by location: the arm defers until the schema document has
	/// been read into the by-location index, and resolves to it after.
	#[crate::test]
	fn document_resolves_by_location() {
		let schema = FieldSchema::document("schema/todo.json");
		schema
			.resolve(SchemaResolver::default())
			.unwrap()
			.xpect_eq(ValueSchema::Any);

		let mut registry = SchemaRegistry::default();
		registry.insert_located("schema/todo.json", ValueSchema::of::<i64>());
		schema
			.resolve(SchemaResolver::default().with_schemas(&registry))
			.unwrap()
			.xpect_eq(ValueSchema::of::<i64>());
	}
}
