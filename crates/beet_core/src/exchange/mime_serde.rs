//! MIME-type-driven serialization and deserialization.
//!
//! Provides free [`serialize`] and [`deserialize`] functions that dispatch
//! based on a [`MimeType`] value. Use these instead of reaching for
//! `serde_json` or `postcard` directly when the format is determined at
//! runtime from a `content-type` header.
//!
//! # Example
//!
//! ```
//! # use beet_core::prelude::*;
//! # use beet_core::exchange::mime_serde;
//! # #[cfg(feature = "json")] {
//! let bytes = mime_serde::serialize(MimeType::Json, &42u32).unwrap();
//! let value: u32 = mime_serde::deserialize(MimeType::Json, &bytes).unwrap();
//! assert_eq!(value, 42);
//! # }
//! ```

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
/// | [`MimeType`]         | Format   | Required feature |
/// |----------------------|----------|-----------------|
/// | `Json`               | JSON     | `json`          |
/// | `Postcard` / `Bytes` | postcard | `postcard`      |
/// | `Text`               | UTF-8    | —               |
#[cfg(feature = "serde")]
pub fn serialize<T: serde::Serialize>(
	mime_type: MimeType,
	value: &T,
) -> Result<Vec<u8>> {
	match mime_type {
		MimeType::Json => {
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
		MimeType::Postcard | MimeType::Bytes => {
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
		other => bevybail!("Cannot serialize to mime type {other}"),
	}
}

/// Deserialize bytes into `T` using the given MIME type's format.
///
/// For [`MimeType::Json`], empty bytes are treated as JSON `null`,
/// enabling unit-type inputs on requests with no body.
///
/// ## Errors
///
/// Returns an error if:
/// - the MIME type is not a supported deserialization format
/// - the bytes fail to deserialize
///
/// ## Supported types
///
/// | [`MimeType`]         | Format   | Required feature |
/// |----------------------|----------|-----------------|
/// | `Json`               | JSON     | `json`          |
/// | `Postcard` / `Bytes` | postcard | `postcard`      |
#[cfg(feature = "serde")]
pub fn deserialize<T: serde::de::DeserializeOwned>(
	mime_type: MimeType,
	bytes: &[u8],
) -> Result<T> {
	match mime_type {
		MimeType::Json => {
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
		MimeType::Postcard | MimeType::Bytes => {
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
		other => bevybail!("Cannot deserialize from mime type {other}"),
	}
}

#[cfg(test)]
#[cfg(feature = "serde")]
mod test {
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
		let bytes = serialize(MimeType::Json, &input).unwrap();
		let output: Pair = deserialize(MimeType::Json, &bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[test]
	fn json_empty_bytes_null() {
		let result: () = deserialize(MimeType::Json, b"").unwrap();
		result.xpect_eq(());
	}

	#[cfg(feature = "postcard")]
	#[test]
	fn roundtrip_postcard() {
		let input = Pair { a: 3, b: 4 };
		let bytes = serialize(MimeType::Postcard, &input).unwrap();
		let output: Pair = deserialize(MimeType::Postcard, &bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "postcard")]
	#[test]
	fn bytes_mime_uses_postcard() {
		let input = Pair { a: 5, b: 6 };
		let bytes = serialize(MimeType::Bytes, &input).unwrap();
		let output: Pair = deserialize(MimeType::Bytes, &bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[test]
	fn unsupported_mime_errors() {
		serialize(MimeType::Html, &42u32).xpect_err();
		serialize(MimeType::Text, &42u32).xpect_err();
	}
}
