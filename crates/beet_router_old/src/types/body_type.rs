//! Body type information for request and response bodies.
//!
//! [`BodyType`] describes how a body is encoded and what Rust type it represents,
//! combining encoding format with type information for documentation and validation.

use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use std::hash::Hash;
use std::hash::Hasher;

/// Describes the body type for a request or response.
///
/// This enum combines encoding format with type information, enabling:
/// - Content-Type header negotiation
/// - API documentation generation
/// - JSON Schema generation for tools
///
/// # Example
///
/// ```ignore
/// use beet_router::prelude::*;
/// use bevy::reflect::Reflect;
///
/// #[derive(Reflect)]
/// struct MyRequest {
///     name: String,
///     count: u32,
/// }
///
/// // JSON body with type info
/// let body = BodyType::json::<MyRequest>();
///
/// // HTML response (no structured type)
/// let html = BodyType::html();
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub enum BodyType {
	/// No body content expected/returned.
	#[default]
	None,
	/// JSON encoding (application/json) with type information.
	Json {
		/// The reflected type information for the body.
		type_info: &'static TypeInfo,
	},
	/// HTML content (text/html) - typically for responses.
	/// Uses String as the underlying type.
	Html,
	/// Binary encoding via bincode (application/octet-stream) with type information.
	Bincode {
		/// The reflected type information for the body.
		type_info: &'static TypeInfo,
	},
	/// Plain text (text/plain).
	/// Uses String as the underlying type.
	Text,
}

impl PartialEq for BodyType {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::None, Self::None) => true,
			(Self::Html, Self::Html) => true,
			(Self::Text, Self::Text) => true,
			(Self::Json { type_info: a }, Self::Json { type_info: b }) => {
				a.type_path() == b.type_path()
			}
			(
				Self::Bincode { type_info: a },
				Self::Bincode { type_info: b },
			) => a.type_path() == b.type_path(),
			_ => false,
		}
	}
}

impl Eq for BodyType {}

impl Hash for BodyType {
	fn hash<H: Hasher>(&self, state: &mut H) {
		// Hash the discriminant first
		core::mem::discriminant(self).hash(state);
		// Then hash the type path for variants that have type info
		match self {
			Self::Json { type_info } | Self::Bincode { type_info } => {
				type_info.type_path().hash(state);
			}
			Self::None | Self::Html | Self::Text => {}
		}
	}
}

impl BodyType {
	/// Creates a body type with no content.
	pub fn none() -> Self { Self::None }

	/// Creates a JSON-encoded body type from a [`Typed`] type.
	pub fn json<T: Typed>() -> Self {
		Self::Json {
			type_info: T::type_info(),
		}
	}

	/// Creates an HTML body type (typically for responses).
	pub fn html() -> Self { Self::Html }

	/// Creates a bincode-encoded body type from a [`Typed`] type.
	pub fn bincode<T: Typed>() -> Self {
		Self::Bincode {
			type_info: T::type_info(),
		}
	}

	/// Creates a plain text body type.
	pub fn text() -> Self { Self::Text }

	/// Returns `true` if this body type has no content.
	pub fn is_none(&self) -> bool { matches!(self, Self::None) }

	/// Returns `true` if this body type is HTML.
	pub fn is_html(&self) -> bool { matches!(self, Self::Html) }

	/// Returns `true` if this body type is JSON.
	pub fn is_json(&self) -> bool { matches!(self, Self::Json { .. }) }

	/// Returns `true` if this body type is bincode.
	pub fn is_bincode(&self) -> bool { matches!(self, Self::Bincode { .. }) }

	/// Returns `true` if this body type is plain text.
	pub fn is_text(&self) -> bool { matches!(self, Self::Text) }

	/// Returns the type information if available.
	///
	/// Returns `Some` for JSON and Bincode variants, `None` for others.
	pub fn type_info(&self) -> Option<&'static TypeInfo> {
		match self {
			Self::Json { type_info } | Self::Bincode { type_info } => {
				Some(type_info)
			}
			Self::None | Self::Html | Self::Text => None,
		}
	}

	/// Returns the encoding name as a string slice.
	pub fn encoding(&self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Json { .. } => "json",
			Self::Html => "html",
			Self::Bincode { .. } => "bincode",
			Self::Text => "text",
		}
	}

	/// Returns the content type header value for this body type.
	pub fn content_type(&self) -> Option<&'static str> {
		match self {
			Self::None => None,
			Self::Json { .. } => Some("application/json"),
			Self::Html => Some("text/html"),
			Self::Bincode { .. } => Some("application/octet-stream"),
			Self::Text => Some("text/plain"),
		}
	}

	/// Returns a short display name for the type.
	///
	/// For JSON and Bincode, returns the reflected type's short name.
	/// For HTML, returns "Html". For Text, returns "String". For None, returns "None".
	pub fn type_display(&self) -> String {
		match self {
			Self::None => "None".to_string(),
			Self::Json { type_info } | Self::Bincode { type_info } => {
				type_info.type_path_table().short_path().to_string()
			}
			Self::Html => "Html".to_string(),
			Self::Text => "String".to_string(),
		}
	}

	/// Returns the full type path if type info is available.
	pub fn type_path(&self) -> Option<&'static str> {
		self.type_info().map(|info| info.type_path())
	}
}

impl std::fmt::Display for BodyType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::None => write!(f, "none"),
			Self::Json { type_info } => {
				write!(f, "{} (json)", type_info.type_path_table().short_path())
			}
			Self::Html => write!(f, "Html (html)"),
			Self::Bincode { type_info } => {
				write!(
					f,
					"{} (bincode)",
					type_info.type_path_table().short_path()
				)
			}
			Self::Text => write!(f, "String (text)"),
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
	fn body_type_none() {
		let body = BodyType::none();
		body.is_none().xpect_true();
		body.type_info().xpect_none();
		body.encoding().xpect_eq("none");
		body.content_type().xpect_none();
	}

	#[test]
	fn body_type_json() {
		let body = BodyType::json::<TestRequest>();
		body.is_json().xpect_true();
		body.is_none().xpect_false();
		body.encoding().xpect_eq("json");
		body.content_type().unwrap().xpect_eq("application/json");

		let type_info = body.type_info().unwrap();
		type_info
			.type_path_table()
			.short_path()
			.xpect_eq("TestRequest");

		body.type_display().xpect_eq("TestRequest");
	}

	#[test]
	fn body_type_html() {
		let body = BodyType::html();
		body.is_html().xpect_true();
		body.type_info().xpect_none();
		body.encoding().xpect_eq("html");
		body.content_type().unwrap().xpect_eq("text/html");
		body.type_display().xpect_eq("Html");
	}

	#[test]
	fn body_type_text() {
		let body = BodyType::text();
		body.is_text().xpect_true();
		body.type_info().xpect_none();
		body.encoding().xpect_eq("text");
		body.content_type().unwrap().xpect_eq("text/plain");
		body.type_display().xpect_eq("String");
	}

	#[test]
	fn body_type_bincode() {
		let body = BodyType::bincode::<TestResponse>();
		body.is_bincode().xpect_true();
		body.encoding().xpect_eq("bincode");
		body.content_type()
			.unwrap()
			.xpect_eq("application/octet-stream");

		let type_info = body.type_info().unwrap();
		type_info
			.type_path_table()
			.short_path()
			.xpect_eq("TestResponse");
	}

	#[test]
	fn display_formatting() {
		let json_body = BodyType::json::<TestRequest>();
		format!("{}", json_body).xpect_eq("TestRequest (json)");

		let none_body = BodyType::none();
		format!("{}", none_body).xpect_eq("none");

		let html_body = BodyType::html();
		format!("{}", html_body).xpect_eq("Html (html)");

		let text_body = BodyType::text();
		format!("{}", text_body).xpect_eq("String (text)");
	}

	#[test]
	fn default_is_none() {
		let body: BodyType = default();
		body.is_none().xpect_true();
	}
}
