use crate::prelude::*;
use alloc::borrow::Cow;


/// A chunk of bytes with an associated media type.
///
/// The inner bytes are stored as a [`Cow`], allowing zero-copy use of borrowed
/// slices alongside owned allocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaBytes<'a> {
	/// The media type of these bytes.
	media_type: MediaType,
	/// The raw bytes of the media content.
	bytes: Cow<'a, [u8]>,
}

impl<'a> MediaBytes<'a> {
	/// Create a new [`MediaBytes`] with the given media type and bytes.
	pub fn new(media_type: MediaType, bytes: impl Into<Cow<'a, [u8]>>) -> Self {
		Self {
			media_type,
			bytes: bytes.into(),
		}
	}

	/// Create a [`MediaBytes`] from a UTF-8 string.
	pub fn from_str(media_type: MediaType, content: &'a str) -> Self {
		Self::new(media_type, content.as_bytes())
	}

	/// Create an owned [`MediaBytes<'static>`] from a [`String`].
	pub fn from_string(
		media_type: MediaType,
		content: String,
	) -> MediaBytes<'static> {
		MediaBytes {
			media_type,
			bytes: Cow::Owned(content.into_bytes()),
		}
	}

	/// The media type of these bytes.
	pub fn media_type(&self) -> &MediaType { &self.media_type }

	/// The raw bytes of the content.
	pub fn bytes(&self) -> &[u8] { &self.bytes }

	/// Try to interpret the bytes as a UTF-8 string slice.
	pub fn as_str(&self) -> Option<&str> {
		core::str::from_utf8(&self.bytes).ok()
	}

	/// Convert into an owned `MediaBytes<'static>`, cloning the bytes if needed.
	pub fn into_owned(self) -> MediaBytes<'static> {
		MediaBytes {
			media_type: self.media_type,
			bytes: Cow::Owned(self.bytes.into_owned()),
		}
	}

	/// Serialize `value` using this media type's format, returning owned [`MediaBytes`].
	///
	/// ## Errors
	///
	/// Returns an error if serialization fails or the media type is not
	/// a supported serialization format.
	#[cfg(feature = "serde")]
	pub fn serialize<T: serde::Serialize>(
		media_type: MediaType,
		value: &T,
	) -> Result<MediaBytes<'static>> {
		let bytes = media_type.serialize(value)?;
		Ok(MediaBytes {
			media_type,
			bytes: Cow::Owned(bytes),
		})
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

impl<'a> core::fmt::Display for MediaBytes<'a> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match core::str::from_utf8(&self.bytes) {
			Ok(text) => write!(f, "{text}"),
			Err(_) => {
				write!(f, "<{} bytes of {}>", self.bytes.len(), self.media_type)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn new_borrowed() {
		let data = b"hello";
		let mb = MediaBytes::new(MediaType::Text, data.as_slice());
		mb.bytes().xpect_eq(b"hello".as_slice());
		mb.media_type().xpect_eq(MediaType::Text);
	}

	#[test]
	fn new_owned() {
		let mb = MediaBytes::new(
			MediaType::Html,
			Vec::from(b"<p>hi</p>".as_slice()),
		);
		mb.as_str().unwrap().xpect_eq("<p>hi</p>");
	}

	#[test]
	fn from_str_ctor() {
		let mb = MediaBytes::from_str(MediaType::Text, "hello");
		mb.as_str().unwrap().xpect_eq("hello");
	}

	#[test]
	fn from_string_ctor() {
		let mb =
			MediaBytes::from_string(MediaType::Html, "<b>bold</b>".to_string());
		mb.as_str().unwrap().xpect_eq("<b>bold</b>");
	}

	#[test]
	fn into_owned() {
		let data = b"data";
		let borrowed = MediaBytes::new(MediaType::Bytes, data.as_slice());
		let owned: MediaBytes<'static> = borrowed.into_owned();
		owned.bytes().xpect_eq(b"data".as_slice());
	}

	#[test]
	fn display_text() {
		MediaBytes::from_str(MediaType::Text, "hello world")
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
}
