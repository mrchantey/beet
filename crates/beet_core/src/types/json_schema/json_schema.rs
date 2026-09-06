//! JSON Schema wrapper and editing operations.
use crate::prelude::*;

/// A JSON Schema represented as a [`Value`].
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, DerefMut, Reflect,
)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct JsonSchema(Value);

impl JsonSchema {
	/// Wraps an existing [`Value`] as a schema.
	pub fn from_value(value: Value) -> Self { Self(value) }

	/// Returns the inner [`Value`].
	pub fn into_inner(self) -> Value { self.0 }
	/// Constrains a top-level string property to a runtime set of allowed values by
	/// injecting a JSON-schema `enum`, so a tool field's options can come from data
	/// (eg a blob-store listing) rather than a fixed Rust enum. Rewrites the property
	/// to `{"type":"string","enum":[..]}`, preserving any sibling keywords like
	/// `description`; a no-op if `field` is absent. Pairs with `StringEnumOptions`
	/// for a reactive, component-driven update.
	pub fn set_field_enum(
		&mut self,
		field: &str,
		options: impl IntoIterator<Item = SmolStr>,
	) -> &mut Self {
		let options = options.into_iter().map(Value::Str).collect::<Vec<_>>();
		if let Some(field_schema) = self
			.get_mut("properties")
			.and_then(|properties| properties.get_mut(field))
			.and_then(|field_schema| field_schema.as_map_mut().ok())
		{
			field_schema.insert("type", "string");
			field_schema.insert("enum", Value::List(options));
		}
		self
	}
}
