//! Media-type-driven serialization and deserialization.
//!
//! Serialize and deserialize values using [`MediaType::serialize`] and
//! [`MediaType::deserialize`]:
//!
//! ```
//! # use beet_core::prelude::*;
//! # #[cfg(feature = "json")] {
//! let bytes = MediaType::Json.serialize(&42u32).unwrap();
//! let value: u32 = MediaType::Json.deserialize(&bytes).unwrap();
//! assert_eq!(value, 42);
//! # }
//! ```
use crate::prelude::*;

impl MediaType {
	/// Serialize `value` into bytes using the first media type in `accept` that
	/// matches, falling back to plaintext if empty.
	pub fn serialize_accepts<T: serde::Serialize>(
		accept: &[MediaType],
		value: &T,
	) -> Result<(MediaType, Vec<u8>)> {
		for media_type in accept {
			if let Ok(bytes) = media_type.serialize(value) {
				return Ok((media_type.clone(), bytes));
			}
		}
		// last resort, see if it accepts text
		if accept.is_empty() {
			let value = serde_plain::to_string(value)?;
			Ok((MediaType::Text, value.into_bytes()))
		} else {
			bevybail!(
				"None of the accept media types could serialize the value\ntypes: {:?}",
				accept
			)
		}
	}


	/// Serialize `value` into bytes using this media type's format.
	///
	/// ## Errors
	///
	/// Returns an error if:
	/// - the media type is not a supported serialization format
	/// - the value fails to serialize
	///
	/// ## Supported types
	///
	/// | [`MediaType`]        | Format   | Required feature |
	/// |----------------------|----------|-----------------|
	/// | `Json`               | JSON     | `json`          |
	/// | `Postcard` / `Bytes` | postcard | `postcard`      |
	#[cfg(feature = "serde")]
	pub fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>> {
		match self {
			MediaType::Text => {
				let value = serde_plain::to_string(value)?;
				Ok(value.into_bytes())
			}
			MediaType::Json => {
				#[cfg(feature = "json")]
				{
					serde_json::to_vec(value).map_err(|err| {
						bevyhow!("Failed to serialize JSON: {err}")
					})
				}
				#[cfg(not(feature = "json"))]
				{
					let _ = value;
					bevybail!(
						"The `json` feature is required for JSON serialization"
					)
				}
			}
			MediaType::Postcard | MediaType::Bytes => {
				#[cfg(feature = "postcard")]
				{
					postcard::to_allocvec(value).map_err(|err| {
						bevyhow!("Failed to serialize postcard: {err}")
					})
				}
				#[cfg(not(feature = "postcard"))]
				{
					let _ = value;
					bevybail!(
						"The `postcard` feature is required for postcard serialization"
					)
				}
			}
			other => bevybail!("Cannot serialize to media type {other}"),
		}
	}

	/// Deserialize bytes into `T` using this media type's format.
	///
	/// For [`MediaType::Json`], empty bytes are treated as JSON `null`,
	/// enabling unit-type inputs on requests with no body.
	///
	/// ## Errors
	///
	/// Returns an error if:
	/// - the media type is not a supported deserialization format
	/// - the bytes fail to deserialize
	///
	/// ## Supported types
	///
	/// | [`MediaType`]        | Format   | Required feature |
	/// |----------------------|----------|-----------------|
	/// | `Json`               | JSON     | `json`          |
	/// | `Postcard` / `Bytes` | postcard | `postcard`      |
	#[cfg(feature = "serde")]
	pub fn deserialize<T: serde::de::DeserializeOwned>(
		&self,
		bytes: &[u8],
	) -> Result<T> {
		match self {
			MediaType::Json => {
				#[cfg(feature = "json")]
				{
					let slice = if bytes.is_empty() { b"null" } else { bytes };
					serde_json::from_slice(slice).map_err(|err| {
						bevyhow!("Failed to deserialize JSON body: {err}")
					})
				}
				#[cfg(not(feature = "json"))]
				{
					let _ = bytes;
					bevybail!(
						"The `json` feature is required for JSON deserialization"
					)
				}
			}
			MediaType::Postcard | MediaType::Bytes => {
				#[cfg(feature = "postcard")]
				{
					postcard::from_bytes(bytes).map_err(|err| {
						bevyhow!("Failed to deserialize postcard body: {err}")
					})
				}
				#[cfg(not(feature = "postcard"))]
				{
					let _ = bytes;
					bevybail!(
						"The `postcard` feature is required for postcard deserialization"
					)
				}
			}
			other => bevybail!("Cannot deserialize from media type {other}"),
		}
	}
}

#[cfg(test)]
mod test {
	#[allow(unused_imports)]
	use super::*;

	#[cfg(any(feature = "json", feature = "postcard"))]
	#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
	struct Pair {
		a: i32,
		b: i32,
	}

	#[cfg(feature = "json")]
	#[test]
	fn roundtrip_json() {
		let input = Pair { a: 1, b: 2 };
		let bytes = MediaType::Json.serialize(&input).unwrap();
		let output: Pair = MediaType::Json.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[test]
	fn json_empty_bytes_null() {
		let result: () = MediaType::Json.deserialize(b"").unwrap();
		result.xpect_eq(());
	}

	#[cfg(feature = "postcard")]
	#[test]
	fn roundtrip_postcard() {
		let input = Pair { a: 3, b: 4 };
		let bytes = MediaType::Postcard.serialize(&input).unwrap();
		let output: Pair = MediaType::Postcard.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "postcard")]
	#[test]
	fn bytes_media_type_uses_postcard() {
		let input = Pair { a: 5, b: 6 };
		let bytes = MediaType::Bytes.serialize(&input).unwrap();
		let output: Pair = MediaType::Bytes.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[test]
	fn unsupported_media_type_errors() {
		MediaType::Html.serialize(&42u32).xpect_err();
		MediaType::Text.serialize(&42u32).xpect_err();
	}
}
