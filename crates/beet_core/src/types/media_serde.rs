//! Media-type-driven serialization and deserialization.
//!
//! Prefer using [`MediaType::serialize`] and [`MediaType::deserialize`]
//! directly instead of the free functions in this module:
//!
//! ```
//! # use beet_core::prelude::*;
//! # #[cfg(feature = "json")] {
//! let bytes = MediaType::Json.serialize(&42u32).unwrap();
//! let value: u32 = MediaType::Json.deserialize(&bytes).unwrap();
//! assert_eq!(value, 42);
//! # }
//! ```
//!
//! The free [`serialize`] and [`deserialize`] functions are kept for
//! cases where a standalone function reference is convenient.
use crate::prelude::*;

/// Serialize `value` into bytes using the given MIME type's format.
///
/// ## Errors
///
/// Returns an error if:
/// - the MIME type is not a supported serialization format
/// - the value fails to serialize
///
/// ## Supported types
///
/// | [`MediaType`]        | Format   | Required feature |
/// |----------------------|----------|-----------------|
/// | `Json`               | JSON     | `json`          |
/// | `Postcard` / `Bytes` | postcard | `postcard`      |
/// | `Text`               | UTF-8    | —               |
pub fn serialize<T: serde::Serialize>(
	media_type: MediaType,
	value: &T,
) -> Result<Vec<u8>> {
	match media_type {
		MediaType::Json => {
			#[cfg(feature = "json")]
			{
				serde_json::to_vec(value)
					.map_err(|err| bevyhow!("Failed to serialize JSON: {err}"))
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

/// Deserialize bytes into `T` using the given media type's format.
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
pub fn deserialize<T: serde::de::DeserializeOwned>(
	media_type: MediaType,
	bytes: &[u8],
) -> Result<T> {
	match media_type {
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
		let bytes = serialize(MediaType::Json, &input).unwrap();
		let output: Pair = deserialize(MediaType::Json, &bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[test]
	fn json_empty_bytes_null() {
		let result: () = deserialize(MediaType::Json, b"").unwrap();
		result.xpect_eq(());
	}

	#[cfg(feature = "postcard")]
	#[test]
	fn roundtrip_postcard() {
		let input = Pair { a: 3, b: 4 };
		let bytes = serialize(MediaType::Postcard, &input).unwrap();
		let output: Pair = deserialize(MediaType::Postcard, &bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "postcard")]
	#[test]
	fn bytes_media_type_uses_postcard() {
		let input = Pair { a: 5, b: 6 };
		let bytes = serialize(MediaType::Bytes, &input).unwrap();
		let output: Pair = deserialize(MediaType::Bytes, &bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[test]
	fn unsupported_media_type_errors() {
		serialize(MediaType::Html, &42u32).xpect_err();
		serialize(MediaType::Text, &42u32).xpect_err();
	}
}
