//! The one vocabulary of primitive kind names, shared by every surface that
//! lets a human write a schema as a single word.
//!
//! Two surfaces read it, and they differ only in what an unrecognized word
//! means, which is the part that must stay deliberate:
//!
//! - a `schema="u64"` markup attribute (`bsx::reflect`), where an unknown word is
//!   an authoring typo and the error lists what is accepted;
//! - a JSON Schema descriptor ([`ValueSchema::from_json_schema`]), where an
//!   unknown word is a [`SchemaRef::Name`] to another schema.
use crate::prelude::*;

impl ValueSchema {
	/// The canonical primitive kind names, as a diagnostic lists them. Only the
	/// markup attribute names them, since only it rejects an unknown word.
	#[cfg(feature = "bsx")]
	pub(crate) const PRIMITIVE_NAMES: &'static str = "\"any\", \"null\", \"bool\", \"i64\", \"u64\", \"f64\", \"string\", \
\"bytes\", \"entity\"";

	/// The unconstrained schema a primitive kind name declares, or `None` when
	/// the word names no primitive.
	///
	/// Both the JSON Schema spelling (`integer`, `number`, `boolean`) and beet's
	/// Rust-type spelling (`i64`, `f64`, `bool`) are accepted, since a schema is
	/// authored by whoever already knows one of the two.
	pub(crate) fn primitive_by_name(name: &str) -> Option<Self> {
		match name {
			"any" => Self::Any,
			"null" => Self::Null,
			"bool" | "boolean" => Self::Bool(default()),
			"i64" | "i32" | "int" | "integer" => Self::I64(default()),
			"u64" | "u32" | "uint" => Self::U64(default()),
			"f64" | "f32" | "float" | "number" => Self::F64(default()),
			"string" | "str" => Self::String(default()),
			"bytes" => Self::Bytes(default()),
			"entity" => Self::Entity(default()),
			_ => return None,
		}
		.xsome()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn accepts_both_spellings() {
		ValueSchema::primitive_by_name("integer")
			.unwrap()
			.xpect_eq(ValueSchema::primitive_by_name("i64").unwrap());
		ValueSchema::primitive_by_name("boolean")
			.unwrap()
			.xpect_eq(ValueSchema::Bool(default()));
		ValueSchema::primitive_by_name("TodoItem").xpect_none();
	}
}
