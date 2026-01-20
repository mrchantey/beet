use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;

/// How a body payload is encoded over the wire.
///
/// This is separate from the body's Rust type - the same struct could be
/// encoded as JSON or Bincode depending on the endpoint configuration.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BodyEncoding {
	/// No body content expected/returned
	#[default]
	None,
	/// JSON encoding (application/json)
	Json,
	/// HTML content (text/html) - typically only for responses
	Html,
	/// Binary encoding via bincode (application/octet-stream)
	Bincode,
	/// Plain text (text/plain)
	Text,
}

impl std::fmt::Display for BodyEncoding {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::None => write!(f, "none"),
			Self::Json => write!(f, "json"),
			Self::Html => write!(f, "html"),
			Self::Bincode => write!(f, "bincode"),
			Self::Text => write!(f, "text"),
		}
	}
}

/// Metadata for a field within a struct body type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldSchema {
	/// Field name
	name: String,
	/// Full type path of the field (e.g., "core::option::Option<alloc::string::String>")
	type_path: String,
	/// Whether this field is required (not wrapped in Option)
	required: bool,
}

impl FieldSchema {
	/// Create a new field schema
	pub fn new(
		name: impl Into<String>,
		type_path: impl Into<String>,
		required: bool,
	) -> Self {
		Self {
			name: name.into(),
			type_path: type_path.into(),
			required,
		}
	}

	/// The field name
	pub fn name(&self) -> &str { &self.name }

	/// The full type path
	pub fn type_path(&self) -> &str { &self.type_path }

	/// Whether this field is required
	pub fn is_required(&self) -> bool { self.required }

	/// Extract from a bevy reflect NamedField
	pub fn from_named_field(field: &bevy::reflect::NamedField) -> Self {
		let type_path = field.type_path();
		let required = !type_path.starts_with("core::option::Option<");
		Self {
			name: field.name().to_string(),
			type_path: type_path.to_string(),
			required,
		}
	}
}

impl std::fmt::Display for FieldSchema {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}: {}", self.name, self.short_type_name())?;
		if self.required {
			write!(f, " (required)")?;
		}
		Ok(())
	}
}

impl FieldSchema {
	/// Get a shortened version of the type name for display
	fn short_type_name(&self) -> String {
		// Extract just the final type name from the full path
		ShortName(&self.type_path).to_string()
	}
}

/// Schema information extracted from a Reflect type.
///
/// This captures the structure of the body type for documentation
/// and validation purposes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypeSchema {
	/// Full type path (e.g., "my_crate::MyStruct")
	type_path: String,
	/// Short name (e.g., "MyStruct")
	short_name: String,
	/// Fields if this is a struct type
	fields: Vec<FieldSchema>,
}

impl TypeSchema {
	/// Create a type schema from a Typed type
	/// ## Panics
	///
	/// Panics if type is not a struct
	pub fn new<T: Typed>() -> Self {
		let type_info = T::type_info();
		let type_path = type_info.type_path().to_string();
		let short_name = type_info.type_path_table().short_path().to_string();

		let fields = match type_info {
			TypeInfo::Struct(struct_info) => struct_info
				.iter()
				.map(FieldSchema::from_named_field)
				.collect(),
			TypeInfo::Tuple(tuple) => {
				if tuple.field_len() == 0 {
					// Unit type
					Vec::new()
				} else {
					panic!("Tuple types not supported, found {type_info:?}")
				}
			}
			type_info => {
				panic!(
					"Only struct and unit types allowed, found {type_info:?}"
				)
			}
		};

		Self {
			type_path,
			short_name,
			fields,
		}
	}

	/// Create a simple schema with just a type name (no field info)
	pub fn simple(type_path: impl Into<String>) -> Self {
		let type_path = type_path.into();
		let short_name = ShortName(&type_path).to_string();
		Self {
			type_path,
			short_name,
			fields: Vec::new(),
		}
	}

	/// The full type path
	pub fn type_path(&self) -> &str { &self.type_path }

	/// The short type name
	pub fn short_name(&self) -> &str { &self.short_name }

	/// The fields if this is a struct
	pub fn fields(&self) -> &[FieldSchema] { &self.fields }

	/// Check if this represents a unit type (no content)
	pub fn is_unit(&self) -> bool { self.type_path == "()" }
}

impl std::fmt::Display for TypeSchema {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.short_name)
	}
}

/// Complete metadata describing a request or response body.
///
/// Combines the wire encoding with type schema information for
/// documentation and validation.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BodyMeta {
	/// How the body is encoded over the wire
	encoding: BodyEncoding,
	/// Type schema, None when type info isn't available
	schema: Option<TypeSchema>,
}

impl BodyMeta {
	/// Create a body meta with no body content
	pub fn none() -> Self {
		Self {
			encoding: BodyEncoding::None,
			schema: None,
		}
	}

	/// Create a JSON-encoded body meta from a Typed type
	pub fn json<T: Typed>() -> Self {
		Self {
			encoding: BodyEncoding::Json,
			schema: Some(TypeSchema::new::<T>()),
		}
	}

	/// Create an HTML body meta (typically for responses)
	pub fn html() -> Self {
		Self {
			encoding: BodyEncoding::Html,
			schema: Some(TypeSchema::simple("Html")),
		}
	}

	/// Create a bincode-encoded body meta from a Typed type
	pub fn bincode<T: Typed>() -> Self {
		Self {
			encoding: BodyEncoding::Bincode,
			schema: Some(TypeSchema::new::<T>()),
		}
	}

	/// Create a plain text body meta
	pub fn text() -> Self {
		Self {
			encoding: BodyEncoding::Text,
			schema: Some(TypeSchema::simple("String")),
		}
	}

	/// Create a body meta with custom encoding and schema
	pub fn new(encoding: BodyEncoding, schema: Option<TypeSchema>) -> Self {
		Self { encoding, schema }
	}

	/// The body encoding
	pub fn encoding(&self) -> BodyEncoding { self.encoding }

	/// The type schema if available
	pub fn schema(&self) -> Option<&TypeSchema> { self.schema.as_ref() }

	/// Check if this represents no body content
	pub fn is_none(&self) -> bool { self.encoding == BodyEncoding::None }

	/// Get a display string for the body type
	pub fn type_display(&self) -> String {
		match &self.schema {
			Some(schema) => schema.short_name().to_string(),
			None => match self.encoding {
				BodyEncoding::None => "None".to_string(),
				BodyEncoding::Html => "Html".to_string(),
				BodyEncoding::Text => "String".to_string(),
				_ => "Unknown".to_string(),
			},
		}
	}
}

impl std::fmt::Display for BodyMeta {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.schema {
			Some(schema) => {
				write!(f, "{} ({})", schema.short_name(), self.encoding)
			}
			None => write!(f, "{}", self.encoding),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Reflect)]
	struct TestRequest {
		name: String,
		count: u32,
		optional_field: Option<String>,
	}

	#[derive(Reflect)]
	struct TestResponse {
		id: u64,
		message: String,
	}

	#[test]
	fn body_meta_none() {
		let meta = BodyMeta::none();
		meta.encoding().xpect_eq(BodyEncoding::None);
		meta.schema().xpect_none();
		meta.is_none().xpect_true();
	}

	#[test]
	fn body_meta_json() {
		let meta = BodyMeta::json::<TestRequest>();
		meta.encoding().xpect_eq(BodyEncoding::Json);

		let schema = meta.schema().unwrap();
		schema.short_name().xpect_eq("TestRequest");
		schema.fields().len().xpect_eq(3);

		// Check field details
		let name_field = &schema.fields()[0];
		name_field.name().xpect_eq("name");
		name_field.is_required().xpect_true();

		let optional_field = &schema.fields()[2];
		optional_field.name().xpect_eq("optional_field");
		optional_field.is_required().xpect_false();
	}

	#[test]
	fn body_meta_html() {
		let meta = BodyMeta::html();
		meta.encoding().xpect_eq(BodyEncoding::Html);
		meta.type_display().xpect_eq("Html");
	}

	#[test]
	fn body_meta_text() {
		let meta = BodyMeta::text();
		meta.encoding().xpect_eq(BodyEncoding::Text);
		meta.type_display().xpect_eq("String");
	}

	#[test]
	fn body_meta_bincode() {
		let meta = BodyMeta::bincode::<TestResponse>();
		meta.encoding().xpect_eq(BodyEncoding::Bincode);

		let schema = meta.schema().unwrap();
		schema.short_name().xpect_eq("TestResponse");
		schema.fields().len().xpect_eq(2);
	}

	#[test]
	fn type_schema_unit() {
		let schema = TypeSchema::new::<()>();
		schema.is_unit().xpect_true();
	}

	#[test]
	fn display_formatting() {
		let meta = BodyMeta::json::<TestRequest>();
		let display = format!("{}", meta);
		display.xpect_eq("TestRequest (json)");

		let none_meta = BodyMeta::none();
		let none_display = format!("{}", none_meta);
		none_display.xpect_eq("none");
	}

	#[test]
	fn field_schema_display() {
		let field = FieldSchema::new("username", "alloc::string::String", true);
		let display = format!("{}", field);
		display.xpect_eq("username: String (required)");
	}
}
