//! Bytes typed by a [`MediaType`]
use crate::prelude::*;

/// Owned bytes paired with a [`MediaType`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Deref, Reflect)]
#[reflect(Default)]
pub struct MediaBytes {
	/// The media type of these bytes.
	media_type: MediaType,
	/// The raw bytes of the media content.
	#[deref]
	bytes: Vec<u8>,
}

impl MediaBytes {
	/// Create a new [`MediaBytes`] with the given media type and bytes.
	pub fn new(media_type: MediaType, bytes: impl Into<Vec<u8>>) -> Self {
		Self {
			media_type,
			bytes: bytes.into(),
		}
	}

	/// Create a [`MediaBytes`] from a UTF-8 string slice.
	pub fn new_str(media_type: MediaType, content: &str) -> Self {
		Self::new(media_type, content.as_bytes().to_vec())
	}

	/// Create a [`MediaBytes`] from an owned [`String`].
	pub fn new_string(media_type: MediaType, content: String) -> Self {
		Self::new(media_type, content.into_bytes())
	}

	/// Create [`MediaBytes`] with [`MediaType::Html`].
	pub fn new_html(html: impl AsRef<str>) -> Self {
		Self::new_str(MediaType::Html, html.as_ref())
	}

	/// Create [`MediaBytes`] with [`MediaType::Text`].
	pub fn new_text(text: impl AsRef<str>) -> Self {
		Self::new_str(MediaType::Text, text.as_ref())
	}

	/// Create [`MediaBytes`] with [`MediaType::Bytes`].
	pub fn new_octet(bytes: impl Into<Vec<u8>>) -> Self {
		Self::new(MediaType::Bytes, bytes)
	}

	/// Create [`MediaBytes`] with [`MediaType::Markdown`].
	pub fn new_markdown(text: impl AsRef<str>) -> Self {
		Self::new_str(MediaType::Markdown, text.as_ref())
	}

	/// Create [`MediaBytes`] with [`MediaType::Json`].
	pub fn new_json(text: impl AsRef<str>) -> Self {
		Self::new_str(MediaType::Json, text.as_ref())
	}

	/// Create [`MediaBytes`] with [`MediaType::Css`].
	pub fn new_css(text: impl AsRef<str>) -> Self {
		Self::new_str(MediaType::Css, text.as_ref())
	}

	/// Create [`MediaBytes`] with [`MediaType::Javascript`].
	pub fn new_javascript(text: impl AsRef<str>) -> Self {
		Self::new_str(MediaType::Javascript, text.as_ref())
	}

	/// The media type of these bytes.
	pub fn media_type(&self) -> &MediaType { &self.media_type }

	/// The raw bytes of the content.
	pub fn bytes(&self) -> &[u8] { &self.bytes }

	/// Try to interpret the bytes as a UTF-8 string slice.
	pub fn as_utf8(&self) -> Result<&str> {
		core::str::from_utf8(&self.bytes)?.xok()
	}

	/// Consume and return the media type and bytes.
	pub fn take(self) -> (MediaType, Vec<u8>) { (self.media_type, self.bytes) }

	/// Serialize `value` using the given media type's format, returning [`MediaBytes`].
	///
	/// ## Errors
	///
	/// Returns an error if serialization fails or the media type is not
	/// a supported serialization format.
	#[cfg(feature = "serde")]
	pub fn serialize<T: serde::Serialize>(
		media_type: MediaType,
		value: &T,
	) -> Result<MediaBytes> {
		let bytes = media_type.serialize(value)?;
		MediaBytes::new(media_type, bytes).xok()
	}

	/// Serialize `value` with the given [`SerializeOptions`].
	#[cfg(feature = "serde")]
	pub fn serialize_with_options<T: serde::Serialize>(
		media_type: MediaType,
		value: &T,
		options: SerializeOptions,
	) -> Result<MediaBytes> {
		let bytes = media_type.serialize_with_options(value, options)?;
		MediaBytes::new(media_type, bytes).xok()
	}

	/// Deserialize bytes into `T` using this media type's format.
	///
	/// ## Errors
	///
	/// Returns an error if deserialization fails or the media type is not
	/// a supported deserialization format.
	#[cfg(feature = "serde")]
	pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
		self.media_type.deserialize(&self.bytes)
	}
}

impl core::fmt::Display for MediaBytes {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match core::str::from_utf8(&self.bytes) {
			Ok(text) => write!(f, "{text}"),
			Err(_) => {
				write!(f, "<{} bytes of {}>", self.bytes.len(), self.media_type)
			}
		}
	}
}

impl Into<bytes::Bytes> for MediaBytes {
	fn into(self) -> bytes::Bytes { self.bytes.into() }
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn new_from_vec() {
		let mb = MediaBytes::new(MediaType::Html, b"<p>hi</p>".to_vec());
		mb.as_utf8().unwrap().xpect_eq("<p>hi</p>");
	}

	#[test]
	fn new_from_slice() {
		let mb = MediaBytes::new(MediaType::Text, b"hello".as_slice());
		mb.bytes().xpect_eq(b"hello".as_slice());
		mb.media_type().xpect_eq(MediaType::Text);
	}

	#[test]
	fn new_str_ctor() {
		let mb = MediaBytes::new_str(MediaType::Text, "hello");
		mb.as_utf8().unwrap().xpect_eq("hello");
	}

	#[test]
	fn new_string_ctor() {
		let mb =
			MediaBytes::new_string(MediaType::Html, "<b>bold</b>".to_string());
		mb.as_utf8().unwrap().xpect_eq("<b>bold</b>");
	}

	#[test]
	fn new_html_helper() {
		let mb = MediaBytes::new_html("<p>hi</p>");
		mb.media_type().xpect_eq(MediaType::Html);
		mb.as_utf8().unwrap().xpect_eq("<p>hi</p>");
	}

	#[test]
	fn new_text_helper() {
		let mb = MediaBytes::new_text("hello");
		mb.media_type().xpect_eq(MediaType::Text);
		mb.as_utf8().unwrap().xpect_eq("hello");
	}

	#[test]
	fn new_octet_helper() {
		let mb = MediaBytes::new_octet(vec![0xFF, 0xFE]);
		mb.media_type().xpect_eq(MediaType::Bytes);
		mb.bytes().xpect_eq(&[0xFF, 0xFE]);
	}

	#[test]
	fn new_markdown_helper() {
		let mb = MediaBytes::new_markdown("# Title");
		mb.media_type().xpect_eq(MediaType::Markdown);
		mb.as_utf8().unwrap().xpect_eq("# Title");
	}

	#[test]
	fn as_utf8_invalid() {
		let mb = MediaBytes::new(MediaType::Bytes, vec![0xFF, 0xFE]);
		mb.as_utf8().xpect_err();
	}

	#[test]
	fn take_returns_parts() {
		let mb = MediaBytes::new_text("data");
		let (media_type, bytes) = mb.take();
		media_type.xpect_eq(MediaType::Text);
		bytes.xpect_eq(b"data".to_vec());
	}

	#[test]
	fn display_text() {
		MediaBytes::new_str(MediaType::Text, "hello world")
			.to_string()
			.xpect_eq("hello world".to_string());
	}

	#[test]
	fn display_binary() {
		MediaBytes::new(MediaType::Bytes, vec![0xFF, 0xFE])
			.to_string()
			.xpect_eq("<2 bytes of application/octet-stream>".to_string());
	}

	#[cfg(feature = "serde")]
	#[cfg(feature = "json")]
	#[test]
	fn serialize_deserialize_roundtrip() {
		#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
		struct Point {
			x: i32,
			y: i32,
		}
		let input = Point { x: 1, y: 2 };
		let mb = MediaBytes::serialize(MediaType::Json, &input).unwrap();
		let output: Point = mb.deserialize().unwrap();
		output.xpect_eq(input);
	}

	#[test]
	fn default_is_empty() {
		let mb = MediaBytes::default();
		mb.media_type().xpect_eq(MediaType::Bytes);
		mb.bytes().xpect_eq(&[] as &[u8]);
	}
}
