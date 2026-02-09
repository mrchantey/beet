//! Content-type based serialization format negotiation.
//!
//! [`ExchangeFormat`] determines the serialization format for
//! request/response bodies based on the `content-type` header.

use crate::prelude::*;

/// The serialization format used for exchange request/response bodies.
///
/// Determined from the `content-type` header, defaulting to JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeFormat {
	/// JSON serialization via `serde_json` (`application/json`).
	Json,
	/// Binary serialization via `postcard` (`application/x-postcard`).
	Postcard,
}

impl ExchangeFormat {
	/// The MIME content-type for JSON.
	pub const JSON_CONTENT_TYPE: &str = "application/json";
	/// The MIME content-type for postcard.
	pub const POSTCARD_CONTENT_TYPE: &str = "application/x-postcard";

	/// Determine the format from a `content-type` header value,
	/// defaulting to JSON if absent.
	///
	/// ## Errors
	///
	/// Returns an error if the content-type is present but unrecognized.
	pub fn from_content_type(content_type: Option<&str>) -> Result<Self> {
		match content_type {
			Some(ct) if ct.contains(Self::POSTCARD_CONTENT_TYPE) => {
				Self::Postcard
			}
			Some(ct) if ct.contains(Self::JSON_CONTENT_TYPE) => Self::Json,
			Some(other) => bevybail!(
				"Unrecognized content-type for exchange: {other}. \
				 Supported: {}, {}.",
				Self::JSON_CONTENT_TYPE,
				Self::POSTCARD_CONTENT_TYPE
			),
			None => Self::Json,
		}
		.xok()
	}

	/// The MIME content-type string for this format.
	pub fn content_type_str(&self) -> &'static str {
		match self {
			Self::Json => Self::JSON_CONTENT_TYPE,
			Self::Postcard => Self::POSTCARD_CONTENT_TYPE,
		}
	}

	/// Deserialize bytes into `T` using this format.
	///
	/// Empty bytes are treated as JSON `null` for the JSON format,
	/// enabling unit-type inputs on requests with no body.
	#[cfg(feature = "serde")]
	pub fn deserialize<T: serde::de::DeserializeOwned>(
		&self,
		bytes: &[u8],
	) -> Result<T> {
		match self {
			Self::Json => {
				#[cfg(feature = "json")]
				{
					let slice =
						if bytes.is_empty() { b"null" } else { bytes };
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
			Self::Postcard => {
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
		}
	}

	/// Serialize `T` into bytes using this format.
	#[cfg(feature = "serde")]
	pub fn serialize<T: serde::Serialize>(
		&self,
		value: &T,
	) -> Result<Vec<u8>> {
		match self {
			Self::Json => {
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
			Self::Postcard => {
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
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use serde::Deserialize;
	use serde::Serialize;

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Pair {
		a: i32,
		b: i32,
	}

	#[test]
	fn from_content_type_default_json() {
		ExchangeFormat::from_content_type(None)
			.unwrap()
			.xpect_eq(ExchangeFormat::Json);
	}

	#[test]
	fn from_content_type_json() {
		ExchangeFormat::from_content_type(Some("application/json"))
			.unwrap()
			.xpect_eq(ExchangeFormat::Json);
	}

	#[test]
	fn from_content_type_json_with_charset() {
		ExchangeFormat::from_content_type(Some(
			"application/json; charset=utf-8",
		))
		.unwrap()
		.xpect_eq(ExchangeFormat::Json);
	}

	#[test]
	fn from_content_type_postcard() {
		ExchangeFormat::from_content_type(Some("application/x-postcard"))
			.unwrap()
			.xpect_eq(ExchangeFormat::Postcard);
	}

	#[test]
	fn from_content_type_unrecognized_errors() {
		ExchangeFormat::from_content_type(Some("text/plain")).xpect_err();
	}

	#[test]
	fn content_type_str() {
		ExchangeFormat::Json
			.content_type_str()
			.xpect_eq("application/json");
		ExchangeFormat::Postcard
			.content_type_str()
			.xpect_eq("application/x-postcard");
	}

	#[cfg(feature = "json")]
	#[test]
	fn roundtrip_json() {
		let input = Pair { a: 1, b: 2 };
		let bytes = ExchangeFormat::Json.serialize(&input).unwrap();
		let output: Pair = ExchangeFormat::Json.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[test]
	fn json_empty_bytes_deserializes_null() {
		let result: () = ExchangeFormat::Json.deserialize(b"").unwrap();
		result.xpect_eq(());
	}

	#[cfg(feature = "postcard")]
	#[test]
	fn roundtrip_postcard() {
		let input = Pair { a: 3, b: 4 };
		let bytes = ExchangeFormat::Postcard.serialize(&input).unwrap();
		let output: Pair =
			ExchangeFormat::Postcard.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}
}
