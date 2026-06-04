use crate::prelude::*;

/// Identifies the value type of a field or token.
///
/// A field schema is either a reference to a Rust [`TypePath`] (resolved at
/// runtime via the [`TypeRegistry`](bevy_reflect::TypeRegistry)), or a fully
/// inlined [`ValueSchema`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FieldSchema {
	/// A Rust [`TypePath`], ie `bevy_color::color::Color`.
	TypePath(SmolStr),
	/// A schema defined inline, without a corresponding registered type.
	Inline(ValueSchema),
}

impl FieldSchema {
	/// Creates a schema from a Rust [`TypePath`].
	pub fn of<T: TypePath>() -> Self {
		Self::TypePath(SmolStr::new_static(T::type_path()))
	}

	/// Creates a schema from an inline [`ValueSchema`].
	pub fn inline(schema: ValueSchema) -> Self { Self::Inline(schema) }

	/// Resolve to a [`ValueSchema`].
	///
	/// `TypePath` variants are looked up in the registry by their Rust type
	/// path. `Inline` variants are returned as-is.
	pub fn resolve(
		&self,
		registry: &bevy_reflect::TypeRegistry,
	) -> Result<ValueSchema> {
		match self {
			Self::Inline(schema) => Ok(schema.clone()),
			Self::TypePath(path) => registry
				.get_with_type_path(path)
				.ok_or_else(|| bevyhow!("type `{}` is not registered", path))?
				.type_info()
				.xmap(ValueSchema::from_type_info)
				.xok(),
		}
	}

	/// Returns the schema's identifying path, or `"inline"` for inline schemas.
	pub fn as_str(&self) -> &str {
		match self {
			Self::TypePath(path) => path.as_str(),
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
		let schema = FieldSchema::inline(inline.clone());
		let registry = bevy_reflect::TypeRegistry::default();
		schema.resolve(&registry).unwrap().xpect_eq(inline);
	}

	#[crate::test]
	fn type_path_resolves_from_registry() {
		let schema = FieldSchema::of::<ResolveTarget>();
		let mut registry = bevy_reflect::TypeRegistry::default();
		registry.register::<ResolveTarget>();
		let resolved = schema.resolve(&registry).unwrap();
		matches!(resolved, ValueSchema::Struct(_)).xpect_true();
	}

	#[crate::test]
	fn type_path_missing_errors() {
		let schema = FieldSchema::of::<ResolveTarget>();
		let registry = bevy_reflect::TypeRegistry::default();
		schema.resolve(&registry).is_err().xpect_true();
	}
}
