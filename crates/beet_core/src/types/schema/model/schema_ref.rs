//! [`SchemaRef`]: the ways a schema can be named rather than written in place.
use crate::prelude::*;

/// How to find a schema that is not written inline.
///
/// The one vocabulary for "described by", reached through
/// [`ValueSchema::Ref`]. Writing a schema in place needs no arm here, because
/// a [`ValueSchema`] *is* the inline case; every other way of naming one is an
/// arm below.
///
/// Three of them resolve against a [`SchemaResolver`] alone, and are what a
/// document uses to declare its own shape:
///
/// - [`Name`](Self::Name), the one [`SchemaRegistry`] namespace, which holds
///   authored and reflect-derived schemas alike
/// - [`TypePath`](Self::TypePath), a registered Rust type, resolved through the
///   by-name registry first so a hand-authored schema can stand in for what
///   reflection would derive
/// - [`Document`](Self::Document), a schema document in this document's own
///   store, resolved by **location** in the by-location index
///
/// The fourth resolves against the *value* instead.
///
/// # `AtField`, the dependent arm
///
/// [`AtField`](Self::AtField) says "my schema is the one described at this key",
/// naming a field of the nearest enclosing struct value, exactly as
/// [`DocumentPath::Ancestor`] names the nearest enclosing document. It is what
/// lets a self-describing pair be written in the schema language rather than
/// hardcoded:
///
/// ```
/// # use beet_core::prelude::*;
/// // `{ "schema": .., "value": .. }`: the value is whatever the sibling says
/// let pair = ValueSchema::Struct(StructSchema {
///     name: Some("TypedDocument".into()),
///     description: None,
///     allow_additional: false,
///     fields: vec![
///         NamedFieldSchema::new("schema", ValueSchema::meta()),
///         NamedFieldSchema::new("value", ValueSchema::at_field("schema")),
///     ],
/// });
/// # let _ = pair;
/// ```
///
/// A nested struct establishes its own scope, so a key always resolves against
/// the innermost struct the field belongs to and never reaches sideways into an
/// unrelated subtree. The value found there is read as a schema: a map as a
/// [`ValueSchema`] (so `{"Bool":{}}` describes a boolean), and a bare string as
/// a [`Name`](Self::Name), which is the discriminator idiom
/// (`{ "kind": "circle", "props": .. }`).
///
/// It yields a *schema*, never a constraint. There is deliberately no way to say
/// "if this field is set then that one is required": that is where a dependent
/// schema stops being resolvable and starts being a rules engine.
///
/// A map whose *keys* name schemas is not a reference at all: that is a
/// [`MapSchema::Keyed`] map, declared on the map rather than in value
/// position.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SchemaRef {
	/// A schema registered under this name in the [`SchemaRegistry`], the one
	/// by-name namespace.
	Name(SmolStr),
	/// A registered Rust type, ie `bevy_color::color::Color`.
	TypePath(SmolStr),
	/// A schema document in this document's own store, resolved by location.
	Document(SmolPath),
	/// The schema described by the value at this key of the nearest enclosing
	/// struct.
	AtField(SmolStr),
}

impl SchemaRef {
	/// The identifying path of this reference, for diagnostics.
	pub fn as_str(&self) -> SmolStr {
		match self {
			Self::Name(name) => name.clone(),
			Self::TypePath(path) => path.clone(),
			Self::Document(path) => path.as_str().into(),
			Self::AtField(key) => key.clone(),
		}
	}

	/// This reference's variant name, ie its externally tagged serde key.
	///
	/// The match is exhaustive, so adding a variant fails to compile until the
	/// meta-schema (which round trips through these names) describes it.
	pub fn variant_name(&self) -> &'static str {
		match self {
			Self::Name(_) => "Name",
			Self::TypePath(_) => "TypePath",
			Self::Document(_) => "Document",
			Self::AtField(_) => "AtField",
		}
	}
}

impl core::fmt::Display for SchemaRef {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Name(name) => write!(f, "{name}"),
			Self::TypePath(path) => write!(f, "type `{path}`"),
			Self::Document(path) => write!(f, "document `{path}`"),
			Self::AtField(key) => write!(f, "the schema at `{key}`"),
		}
	}
}

impl ValueSchema {
	/// A schema registered under `name`, the by-name arm.
	pub fn reference(name: impl Into<SmolStr>) -> Self {
		Self::Ref(SchemaRef::Name(name.into()))
	}

	/// The schema of a registered Rust type, resolved at runtime.
	pub fn type_ref<T: TypePath>() -> Self {
		Self::Ref(SchemaRef::TypePath(SmolStr::new_static(T::type_path())))
	}

	/// The schema held by the schema document at `path`, resolved by location in
	/// this document's own store.
	pub fn document(path: impl Into<SmolPath>) -> Self {
		Self::Ref(SchemaRef::Document(path.into()))
	}

	/// The schema described by the value at `key` of the nearest enclosing
	/// struct, the dependent arm ([`SchemaRef::AtField`]).
	pub fn at_field(key: impl Into<SmolStr>) -> Self {
		Self::Ref(SchemaRef::AtField(key.into()))
	}

	/// The reference this schema is, if it is one.
	pub fn as_ref(&self) -> Option<&SchemaRef> {
		match self {
			Self::Ref(schema_ref) => Some(schema_ref),
			_ => None,
		}
	}
}
