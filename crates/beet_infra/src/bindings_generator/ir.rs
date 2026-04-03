//! Intermediate representation for Terraform schema → Rust codegen.

use std::collections::BTreeMap;

/// A qualified name: `(Option<namespace>, name)`.
///
/// When a namespace is present it typically corresponds to a Terraform
/// block-type grouping (e.g. `"resource"`).  The emitter joins namespace
/// and name with `_` to form the Rust struct name, and emits a
/// `#[serde(rename = "name")]` attribute so that (de)serialisation still
/// uses the short name.
pub type QualifiedName = (Option<String>, String);

/// The central registry that maps qualified names to their container
/// definitions.  Iteration order matters (the emitter writes containers in
/// `BTreeMap` order), so we use a `BTreeMap`.
pub type Registry = BTreeMap<QualifiedName, Container>;

// ---------------------------------------------------------------------------
// Field types
// ---------------------------------------------------------------------------

/// A Rust type that a struct field or enum variant payload can have.
///
/// This mirrors the subset of `serde_reflection::Format` that the Terraform
/// schema actually produces, plus the full set of primitives so that the
/// emitter never needs to fall back to stringly-typed output.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldType {
	Unit,
	Bool,
	I8,
	I16,
	I32,
	I64,
	I128,
	U8,
	U16,
	U32,
	U64,
	U128,
	F32,
	F64,
	Char,
	Str,
	Bytes,
	Option(Box<FieldType>),
	Seq(Box<FieldType>),
	Map {
		key: Box<FieldType>,
		value: Box<FieldType>,
	},
	Tuple(Vec<FieldType>),
	TupleArray {
		content: Box<FieldType>,
		size: usize,
	},
	/// Reference to another named container in the registry.
	TypeName(String),
}

// ---------------------------------------------------------------------------
// Field metadata from Terraform schema attributes
// ---------------------------------------------------------------------------

/// Terraform schema attribute flags propagated through code generation.
///
/// Used to enrich doc comments and generate runtime validation
/// in [`validate_definition`](crate::terra::Resource::validate_definition).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FieldMetadata {
	/// The user must supply this field.
	pub required: bool,
	/// The user may supply this field.
	pub optional: bool,
	/// The provider computes this field (possibly in addition to user input).
	pub computed: bool,
	/// The field contains sensitive data (passwords, keys, etc.).
	pub sensitive: bool,
}

impl FieldMetadata {
	/// A field that is computed by the provider and should not be set by the
	/// user (computed=true, not optional, not required).
	pub fn is_computed_only(&self) -> bool {
		self.computed && !self.optional && !self.required
	}

	/// Format the active flags as a comma-separated doc string,
	/// e.g. `` `required`, `sensitive` ``.
	/// Returns `None` when no flags are set.
	pub fn flags_doc(&self) -> Option<String> {
		let mut parts = Vec::new();
		if self.required {
			parts.push("`required`");
		}
		if self.optional {
			parts.push("`optional`");
		}
		if self.computed {
			parts.push("`computed`");
		}
		if self.sensitive {
			parts.push("`sensitive`");
		}
		if parts.is_empty() {
			None
		} else {
			Some(parts.join(", "))
		}
	}
}

// ---------------------------------------------------------------------------
// Fields & variants
// ---------------------------------------------------------------------------

/// A named, typed field inside a struct (or struct variant).
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
	pub name: String,
	pub value: FieldType,
	/// Terraform schema attribute flags. Defaults to all-false for fields
	/// that don't originate from a Terraform attribute (e.g. block-type
	/// children or manually-constructed IR).
	pub metadata: FieldMetadata,
}

/// The payload shape of a single enum variant.
#[derive(Clone, Debug, PartialEq)]
pub enum VariantFormat {
	Unit,
	NewType(Box<FieldType>),
	Tuple(Vec<FieldType>),
	Struct(Vec<Field>),
}

/// A named enum variant with its payload.
#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
	pub name: String,
	pub value: VariantFormat,
}

// ---------------------------------------------------------------------------
// Containers
// ---------------------------------------------------------------------------

/// A top-level type definition (struct or enum).
#[derive(Clone, Debug, PartialEq)]
pub enum Container {
	UnitStruct,
	NewTypeStruct(FieldType),
	TupleStruct(Vec<FieldType>),
	Struct(Vec<Field>),
	/// Variants keyed by their positional index (used to guarantee
	/// deterministic ordering in the generated code).
	Enum(BTreeMap<u32, Variant>),
}

// ---------------------------------------------------------------------------
// Convenience constructors
// ---------------------------------------------------------------------------

impl Field {
	/// Shorthand for creating a [`Field`] with default (empty) metadata.
	pub fn new(name: impl Into<String>, value: FieldType) -> Self {
		Self {
			name: name.into(),
			value,
			metadata: FieldMetadata::default(),
		}
	}

	/// Create a [`Field`] with explicit metadata.
	pub fn with_metadata(
		name: impl Into<String>,
		value: FieldType,
		metadata: FieldMetadata,
	) -> Self {
		Self {
			name: name.into(),
			value,
			metadata,
		}
	}
}

impl Variant {
	/// Shorthand for creating a [`Variant`].
	pub fn new(name: impl Into<String>, value: VariantFormat) -> Self {
		Self {
			name: name.into(),
			value,
		}
	}
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

impl FieldType {
	/// Returns `true` when this type is `Option<_>`.
	pub fn is_optional(&self) -> bool { matches!(self, FieldType::Option(_)) }
}

impl Container {
	/// If this is a `Struct`, return its fields.
	pub fn fields(&self) -> Option<&[Field]> {
		match self {
			Container::Struct(fields) => Some(fields),
			_ => None,
		}
	}
}
