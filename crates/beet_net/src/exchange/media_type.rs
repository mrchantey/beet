//! Media type classification for HTTP content negotiation.
//!
//! [`MediaType`] represents common IANA media types used in HTTP
//! `Content-Type` and `Accept` headers. The term "media type" is the
//! current IANA standard, replacing the older "MIME type" terminology.

use beet_core::prelude::*;

/// Common media types used in HTTP exchange.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum MediaType {
	/// `application/octet-stream` — raw bytes, the default.
	#[default]
	Bytes,
	/// `text/plain`
	Text,
	/// `text/html`
	Html,
	/// `application/xml` or `text/xml`
	Xml,
	/// `application/json`
	Json,
	/// `application/x-postcard`
	Postcard,
	/// `text/markdown`
	Markdown,
	/// `text/event-stream` — Server-Sent Events.
	EventStream,
	/// `text/css`
	Css,
	/// `application/javascript`
	Javascript,
	/// `image/png`
	Png,
	/// An unrecognized media type.
	Other(String),
}

impl MediaType {
	/// The media type string for `application/json`.
	const JSON: &'static str = "application/json";
	/// The media type string for `application/x-postcard`.
	const POSTCARD: &'static str = "application/x-postcard";
	/// The media type string for `text/plain`.
	const TEXT: &'static str = "text/plain";
	/// The media type string for `text/html`.
	const HTML: &'static str = "text/html";
	/// The media type string for `application/xml`.
	const XML: &'static str = "application/xml";
	/// The media type string for `text/markdown`.
	const MARKDOWN: &'static str = "text/markdown";
	/// The media type string for `application/octet-stream`.
	const BYTES: &'static str = "application/octet-stream";
	/// The media type string for `text/event-stream`.
	const EVENT_STREAM: &'static str = "text/event-stream";
	/// The media type string for `text/css`.
	const CSS: &'static str = "text/css";
	/// The media type string for `application/javascript`.
	const JAVASCRIPT: &'static str = "application/javascript";
	/// The media type string for `image/png`.
	const PNG: &'static str = "image/png";

	/// Parse a media type from a content-type string.
	///
	/// Strips parameters like `; charset=utf-8` before matching.
	pub fn from_content_type(content_type: &str) -> Self {
		let raw = content_type
			.split(';')
			.next()
			.unwrap_or(content_type)
			.trim();
		match raw {
			val if val.contains(Self::JSON) => MediaType::Json,
			val if val.contains(Self::POSTCARD) => MediaType::Postcard,
			val if val.contains(Self::HTML) => MediaType::Html,
			val if val.contains(Self::MARKDOWN) => MediaType::Markdown,
			val if val.contains(Self::EVENT_STREAM) => MediaType::EventStream,
			val if val.contains("text/xml") || val.contains(Self::XML) => {
				MediaType::Xml
			}
			val if val.contains(Self::TEXT) => MediaType::Text,
			val if val.contains(Self::BYTES) => MediaType::Bytes,
			val if val.contains(Self::CSS) => MediaType::Css,
			val if val.contains(Self::JAVASCRIPT) => MediaType::Javascript,
			val if val.contains(Self::PNG) => MediaType::Png,
			other => MediaType::Other(other.to_string()),
		}
	}

	/// The canonical media type string for this type.
	pub fn as_str(&self) -> &str {
		match self {
			MediaType::Bytes => Self::BYTES,
			MediaType::Text => Self::TEXT,
			MediaType::Html => Self::HTML,
			MediaType::Xml => Self::XML,
			MediaType::Json => Self::JSON,
			MediaType::Postcard => Self::POSTCARD,
			MediaType::Markdown => Self::MARKDOWN,
			MediaType::EventStream => Self::EVENT_STREAM,
			MediaType::Css => Self::CSS,
			MediaType::Javascript => Self::JAVASCRIPT,
			MediaType::Png => Self::PNG,
			MediaType::Other(val) => val.as_str(),
		}
	}

	/// Whether this is a serializable format (JSON or Postcard).
	pub fn is_serializable(&self) -> bool {
		matches!(self, MediaType::Json | MediaType::Postcard)
	}
}

impl From<&str> for MediaType {
	fn from(value: &str) -> Self { MediaType::from_content_type(value) }
}

impl From<String> for MediaType {
	fn from(value: String) -> Self { MediaType::from_content_type(&value) }
}

impl Into<Vec<MediaType>> for MediaType {
	fn into(self) -> Vec<MediaType> { vec![self] }
}

impl core::fmt::Display for MediaType {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(formatter, "{}", self.as_str())
	}
}

/// Deprecated alias for [`MediaType`].
#[allow(missing_docs)]
pub type MimeType = MediaType;

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn from_content_type_json() {
		MediaType::from_content_type("application/json")
			.xpect_eq(MediaType::Json);
	}

	#[test]
	fn from_content_type_json_with_charset() {
		MediaType::from_content_type("application/json; charset=utf-8")
			.xpect_eq(MediaType::Json);
	}

	#[test]
	fn from_content_type_postcard() {
		MediaType::from_content_type("application/x-postcard")
			.xpect_eq(MediaType::Postcard);
	}

	#[test]
	fn from_content_type_html() {
		MediaType::from_content_type("text/html").xpect_eq(MediaType::Html);
	}

	#[test]
	fn from_content_type_text() {
		MediaType::from_content_type("text/plain").xpect_eq(MediaType::Text);
	}

	#[test]
	fn from_content_type_xml() {
		MediaType::from_content_type("application/xml")
			.xpect_eq(MediaType::Xml);
		MediaType::from_content_type("text/xml").xpect_eq(MediaType::Xml);
	}

	#[test]
	fn from_content_type_markdown() {
		MediaType::from_content_type("text/markdown")
			.xpect_eq(MediaType::Markdown);
	}

	#[test]
	fn from_content_type_bytes() {
		MediaType::from_content_type("application/octet-stream")
			.xpect_eq(MediaType::Bytes);
	}

	#[test]
	fn from_content_type_event_stream() {
		MediaType::from_content_type("text/event-stream")
			.xpect_eq(MediaType::EventStream);
	}

	#[test]
	fn from_content_type_unknown() {
		MediaType::from_content_type("application/x-custom")
			.xpect_eq(MediaType::Other("application/x-custom".to_string()));
	}

	#[test]
	fn as_str_roundtrip() {
		let types = vec![
			MediaType::Bytes,
			MediaType::Text,
			MediaType::Html,
			MediaType::Xml,
			MediaType::Json,
			MediaType::Postcard,
			MediaType::Markdown,
			MediaType::EventStream,
		];
		for media_type in types {
			MediaType::from_content_type(media_type.as_str())
				.xpect_eq(media_type);
		}
	}

	#[test]
	fn default_is_bytes() { MediaType::default().xpect_eq(MediaType::Bytes); }

	#[test]
	fn display() {
		format!("{}", MediaType::Json).xpect_eq("application/json");
		format!("{}", MediaType::EventStream).xpect_eq("text/event-stream");
	}

	#[test]
	fn is_serializable() {
		MediaType::Json.is_serializable().xpect_true();
		MediaType::Postcard.is_serializable().xpect_true();
		MediaType::Html.is_serializable().xpect_false();
		MediaType::Text.is_serializable().xpect_false();
		MediaType::EventStream.is_serializable().xpect_false();
	}
}
