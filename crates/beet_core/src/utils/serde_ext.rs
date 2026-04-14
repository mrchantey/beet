//! Utilities for serde serialization and deserialization.
use serde::Deserializer;
use serde::Serializer;
use serde::de::Visitor;
use serde::de::{
	self,
};
use std::fmt;

/// Serialize a byte array as either a UTF-8 string (if valid) or as raw bytes.
pub fn serialize_bytes_or_string<S>(
	bytes: &Vec<u8>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match std::str::from_utf8(bytes) {
		Ok(s) => serializer.serialize_str(s),
		Err(_) => serializer.serialize_bytes(bytes),
	}
}

/// Deserialize a byte array from either a UTF-8 string or raw bytes.
pub fn deserialize_bytes_or_string<'de, D>(
	deserializer: D,
) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	struct BytesVisitor;

	impl<'de> Visitor<'de> for BytesVisitor {
		type Value = Vec<u8>;

		fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
			write!(f, "string or byte array")
		}

		fn visit_str<E>(self, v: &str) -> Result<Vec<u8>, E>
		where
			E: de::Error,
		{
			Ok(v.as_bytes().to_vec())
		}

		fn visit_bytes<E>(self, v: &[u8]) -> Result<Vec<u8>, E>
		where
			E: de::Error,
		{
			Ok(v.to_vec())
		}

		fn visit_seq<A>(self, mut seq: A) -> Result<Vec<u8>, A::Error>
		where
			A: de::SeqAccess<'de>,
		{
			let mut out = Vec::new();
			while let Some(b) = seq.next_element()? {
				out.push(b);
			}
			Ok(out)
		}
	}

	deserializer.deserialize_any(BytesVisitor)
}
