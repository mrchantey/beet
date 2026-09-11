//! Composite schema declarations used by [`ValueSchema`].
use crate::prelude::*;

/// A field within a [`StructSchema`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NamedFieldSchema {
	/// The map key for this field.
	pub key: SmolStr,
	/// Whether this field must be present.
	pub required: bool,
	/// Optional human readable label, falls back to `key` if missing.
	pub label: Option<SmolStr>,
	/// Optional description.
	pub description: Option<SmolStr>,
	/// How a schema commit resolves this field when the existing data has no
	/// value for it. `None` declares no resolution, so a commit that would
	/// leave the field required-but-absent is rejected.
	pub on_missing: Option<OnMissing>,
	/// The field's value schema.
	pub schema: ValueSchema,
}

impl NamedFieldSchema {
	/// A required field of the given key and schema, with no resolution policy.
	pub fn new(key: impl Into<SmolStr>, schema: ValueSchema) -> Self {
		Self {
			key: key.into(),
			required: true,
			label: None,
			description: None,
			on_missing: None,
			schema,
		}
	}

	/// Mark the field optional, so an absent value validates.
	pub fn optional(mut self) -> Self {
		self.required = false;
		self
	}

	/// Set the human readable label a generated form shows in place of the key.
	pub fn with_label(mut self, label: impl Into<SmolStr>) -> Self {
		self.label = Some(label.into());
		self
	}

	/// Declare how a schema commit resolves an absent value.
	pub fn with_on_missing(mut self, on_missing: OnMissing) -> Self {
		self.on_missing = Some(on_missing);
		self
	}
}

/// A field within a [`TupleSchema`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnnamedFieldSchema {
	/// Whether this field must be present.
	pub required: bool,
	/// Optional description.
	pub description: Option<SmolStr>,
	/// The field's value schema.
	pub schema: ValueSchema,
}

/// Schema for a struct-shaped value.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StructSchema {
	/// The type's short name, if known.
	pub name: Option<SmolStr>,
	/// The type's own documentation, if any.
	pub description: Option<SmolStr>,
	/// Whether keys not in [`fields`](Self::fields) are permitted.
	pub allow_additional: bool,
	/// Field schemas.
	pub fields: Vec<NamedFieldSchema>,
}

/// Schema for a fixed-arity tuple value, also used for tuple structs.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TupleSchema {
	/// The type's short name, if known.
	pub name: Option<SmolStr>,
	/// The type's own documentation, if any.
	pub description: Option<SmolStr>,
	/// Field schemas in order.
	pub fields: Vec<UnnamedFieldSchema>,
}

/// Schema for a homogenous list value, also used for arrays and sets.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListSchema {
	/// The schema each element must satisfy.
	pub item: Box<ValueSchema>,
	/// Minimum number of elements.
	pub min_items: Option<usize>,
	/// Maximum number of elements.
	pub max_items: Option<usize>,
	/// Whether duplicate elements are forbidden.
	pub unique: bool,
}

/// Schema for a map value with string keys: either every entry satisfies one
/// schema, or each entry satisfies the schema registered under its key.
///
/// A [`Keyed`](Self::Keyed) map is how a component map is described at the
/// schema level rather than special-cased in a widget:
/// `{ "bevy_ecs::name::Name": .., "bevy_ecs::hierarchy::ChildOf": .. }`
/// validates each value against whatever the [`SchemaRegistry`] holds under its
/// key, and a form's add-entry key input is a picker over the registry. A key
/// the registry does not hold is an error
/// ([`entry_schema`](Self::entry_schema)), as an unregistered component is to
/// the template loader.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MapSchema {
	/// Every entry satisfies `value`.
	Uniform {
		/// The schema each value in the map must satisfy.
		value: Box<ValueSchema>,
	},
	/// Each key is a [`SchemaRegistry`] name and its entry satisfies the schema
	/// registered under it.
	Keyed,
}

impl Default for MapSchema {
	fn default() -> Self { Self::uniform(ValueSchema::Any) }
}

impl MapSchema {
	/// A map whose every entry satisfies `value`.
	pub fn uniform(value: ValueSchema) -> Self {
		Self::Uniform {
			value: Box::new(value),
		}
	}

	/// This map's variant name, ie its externally tagged serde key.
	///
	/// The match is exhaustive, so adding a variant fails to compile until the
	/// meta-schema (which round trips through these names) describes it.
	pub fn variant_name(&self) -> &'static str {
		match self {
			Self::Uniform { .. } => "Uniform",
			Self::Keyed => "Keyed",
		}
	}
}

/// A variant within an [`EnumSchema`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VariantSchema {
	/// The variant's name as it appears in serialized form.
	pub name: SmolStr,
	/// Optional payload schema; `None` for unit variants.
	pub payload: Option<ValueSchema>,
}

/// Schema for an enum value, externally tagged: `{"VariantName": payload}` or
/// the bare string `"VariantName"` for unit variants.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EnumSchema {
	/// The type's short name, if known.
	pub name: Option<SmolStr>,
	/// The type's own documentation, if any.
	pub description: Option<SmolStr>,
	/// Variants in declaration order.
	pub variants: Vec<VariantSchema>,
}
