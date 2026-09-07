//! [`ValueSchema`]: an interface-oriented schema for [`Value`]s.
use crate::prelude::*;

/// An interface-oriented description of a [`Value`]'s shape.
///
/// Used for driving dynamic UIs, performing validation and exporting a
/// [`JsonSchema`] representation.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
#[reflect(opaque)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValueSchema {
	/// Matches any value. An escape hatch that disables validation and
	/// type-checking for this field.
	Any,
	/// Always [`Value::Null`].
	Null,
	/// A boolean value.
	Bool(BoolSchema),
	/// A signed 64-bit integer.
	I64(I64Schema),
	/// An unsigned 64-bit integer.
	U64(U64Schema),
	/// A 64-bit float.
	F64(F64Schema),
	/// A string.
	String(StringSchema),
	/// Raw bytes.
	Bytes(BytesSchema),
	/// A reference to another node, ie an [`Entity`].
	Entity(EntitySchema),
	/// A struct with named fields.
	Struct(StructSchema),
	/// A fixed-arity tuple (also used for tuple structs).
	Tuple(TupleSchema),
	/// A homogenous sequence (list, array or set).
	List(ListSchema),
	/// A map with string keys.
	Map(MapSchema),
	/// A tagged union.
	Enum(EnumSchema),
	/// An optional value: [`Value::Null`] is accepted, anything else is
	/// validated against the inner schema. This is how an `Option`-typed field
	/// is represented so a missing or null value validates rather than failing.
	Optional(Box<ValueSchema>),
	/// A schema named rather than written in place, by any of the ways a
	/// [`SchemaRef`] can name one.
	///
	/// This is what makes schemas composable: an `items` array of `TodoItem`
	/// names `TodoItem`'s schema, so schemas form a graph mirroring the template
	/// graph and validate recursively. It is also how a document declares its
	/// own shape, and how a field says it is described by a sibling.
	/// Until resolved, validation against it is a wildcard (deferred), since the
	/// named schema may resolve asynchronously.
	Ref(SchemaRef),
}

impl core::fmt::Display for ValueSchema {
	/// The name a diagnostic calls this schema: what it names when it names
	/// something, else the name it declares for itself, else its kind.
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Ref(schema_ref) => write!(f, "{schema_ref}"),
			schema => match schema.name() {
				Some(name) => write!(f, "{name}"),
				None => write!(f, "{}", schema.variant_name()),
			},
		}
	}
}

impl Default for ValueSchema {
	fn default() -> Self { Self::Null }
}


impl ValueSchema {
	/// This schema's variant name, ie its externally tagged serde key and the
	/// kind a diagnostic names.
	///
	/// The match is exhaustive, so adding a variant fails to compile until the
	/// meta-schema (which round trips through these names) describes it.
	pub fn variant_name(&self) -> &'static str {
		match self {
			Self::Any => "Any",
			Self::Null => "Null",
			Self::Bool(_) => "Bool",
			Self::I64(_) => "I64",
			Self::U64(_) => "U64",
			Self::F64(_) => "F64",
			Self::String(_) => "String",
			Self::Bytes(_) => "Bytes",
			Self::Entity(_) => "Entity",
			Self::Struct(_) => "Struct",
			Self::Tuple(_) => "Tuple",
			Self::List(_) => "List",
			Self::Map(_) => "Map",
			Self::Enum(_) => "Enum",
			Self::Optional(_) => "Optional",
			Self::Ref(_) => "Ref",
		}
	}

	/// Whether this schema describes a value with *parts*: a struct, tuple,
	/// list, map or enum.
	///
	/// The kinds a walk can descend through, and so the ones a depth budget is
	/// spent on; an `Optional` or a [`ValueSchema::Ref`] is the same value seen
	/// more precisely rather than a level of it.
	pub fn is_composite(&self) -> bool {
		matches!(
			self,
			Self::Struct(_)
				| Self::Tuple(_)
				| Self::List(_)
				| Self::Map(_)
				| Self::Enum(_)
		)
	}

	/// The name a composite schema declares for itself, if any.
	///
	/// The authored equivalent of a Rust type's short path, and the key an
	/// authored schema joins the one by-name namespace under.
	pub fn name(&self) -> Option<&SmolStr> {
		match self {
			ValueSchema::Struct(schema) => schema.name.as_ref(),
			ValueSchema::Tuple(schema) => schema.name.as_ref(),
			ValueSchema::Enum(schema) => schema.name.as_ref(),
			_ => None,
		}
	}

	/// The documentation a composite schema carries for itself, if any.
	///
	/// The type-level twin of a field's description: what the Rust type's doc
	/// comment said, or what an authored schema wrote in its place.
	pub fn description(&self) -> Option<&SmolStr> {
		match self {
			ValueSchema::Struct(schema) => schema.description.as_ref(),
			ValueSchema::Tuple(schema) => schema.description.as_ref(),
			ValueSchema::Enum(schema) => schema.description.as_ref(),
			_ => None,
		}
	}
}
