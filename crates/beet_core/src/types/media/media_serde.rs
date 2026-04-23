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

/// Options for controlling serialization output.
#[derive(Debug, Clone, Default)]
pub struct SerializeOptions {
	/// Serialize in a pretty format where possible.
	pub pretty: bool,
}

impl MediaType {
	/// Serialize `value` into [`MediaBytes`] using the first media type in
	/// `accept` that succeeds, falling back to JSON or plain text if empty.
	pub fn serialize_accepts<T: serde::Serialize>(
		accept: &[MediaType],
		value: &T,
	) -> Result<MediaBytes> {
		Self::serialize_accepts_with_options(accept, value, default())
	}

	/// Serialize `value` into [`MediaBytes`] using the first media type in
	/// `accept` that succeeds, with the given [`SerializeOptions`].
	pub fn serialize_accepts_with_options<T: serde::Serialize>(
		accept: &[MediaType],
		value: &T,
		options: SerializeOptions,
	) -> Result<MediaBytes> {
		for media_type in accept {
			if let Ok(bytes) =
				media_type.serialize_with_options(value, options.clone())
			{
				return Ok(MediaBytes::new(media_type.clone(), bytes));
			}
		}
		// last resort fallback
		if accept.is_empty() {
			cfg_if! {
				if #[cfg(feature = "json")]{
					let value = if options.pretty {
						serde_json::to_string_pretty(value)?
					} else {
						serde_json::to_string(value)?
					};
					Ok(MediaBytes::new(MediaType::Json, value.into_bytes()))
				}else {
					let value = serde_plain::to_string(value)?;
					Ok(MediaBytes::new(MediaType::Text, value.into_bytes()))
				}
			}
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
	/// | `Text`               | plain    | always          |
	/// | `Json`               | JSON     | `json`          |
	/// | `Ron`                | RON      | `serde` (includes `ron`) |
	/// | `Postcard` / `Bytes` | postcard | `postcard`      |
	#[cfg(feature = "serde")]
	pub fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>> {
		self.serialize_with_options(value, default())
	}

	/// Serialize `value` into bytes using this media type's format,
	/// with the given [`SerializeOptions`].
	#[cfg(feature = "serde")]
	pub fn serialize_with_options<T: serde::Serialize>(
		&self,
		value: &T,
		options: SerializeOptions,
	) -> Result<Vec<u8>> {
		match self {
			MediaType::Text => {
				let value = serde_plain::to_string(value)?;
				Ok(value.into_bytes())
			}
			MediaType::Json => {
				#[cfg(feature = "json")]
				{
					if options.pretty {
						serde_json::to_vec_pretty(value).map_err(|err| {
							bevyhow!("Failed to serialize JSON: {err}")
						})
					} else {
						serde_json::to_vec(value).map_err(|err| {
							bevyhow!("Failed to serialize JSON: {err}")
						})
					}
				}
				#[cfg(not(feature = "json"))]
				{
					let _ = (value, options);
					bevybail!(
						"The `json` feature is required for JSON serialization"
					)
				}
			}
			MediaType::Ron => {
				if options.pretty {
					let pretty_config = ron::ser::PrettyConfig::default()
						.indentor("  ".to_string())
						.new_line("\n".to_string());
					ron::ser::to_string_pretty(value, pretty_config)
						.map(|s| s.into_bytes())
						.map_err(|err| {
							bevyhow!("Failed to serialize RON: {err}")
						})
				} else {
					ron::ser::to_string(value).map(|s| s.into_bytes()).map_err(
						|err| bevyhow!("Failed to serialize RON: {err}"),
					)
				}
			}
			MediaType::Postcard | MediaType::Bytes => {
				#[cfg(feature = "postcard")]
				{
					let _ = options;
					postcard::to_allocvec(value).map_err(|err| {
						bevyhow!("Failed to serialize postcard: {err}")
					})
				}
				#[cfg(not(feature = "postcard"))]
				{
					let _ = (value, options);
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
	/// | `Text`               | plain    | always          |
	/// | `Json`               | JSON     | `json`          |
	/// | `Ron`                | RON      | `serde` (includes `ron`) |
	/// | `Postcard` / `Bytes` | postcard | `postcard`      |
	#[cfg(feature = "serde")]
	pub fn deserialize<T: serde::de::DeserializeOwned>(
		&self,
		bytes: &[u8],
	) -> Result<T> {
		match self {
			MediaType::Text => {
				let string = std::str::from_utf8(bytes)?;
				serde_plain::from_str(string).map_err(|err| {
					bevyhow!("Failed to deserialize plaintext body: {err}")
				})
			}
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
			MediaType::Ron => {
				let string = std::str::from_utf8(bytes).map_err(|err| {
					bevyhow!("RON data is not valid UTF-8: {err}")
				})?;
				ron::de::from_str(string).map_err(|err| {
					bevyhow!("Failed to deserialize RON body: {err}")
				})
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

	#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
	struct Pair {
		a: i32,
		b: i32,
	}


	#[test]
	fn roundtrip_plaintext() {
		let input: u32 = 20;
		let bytes = MediaType::Text.serialize(&input).unwrap();
		let output: u32 = MediaType::Text.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
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
	fn json_pretty() {
		let input = Pair { a: 1, b: 2 };
		let options = SerializeOptions { pretty: true };
		let bytes = MediaType::Json
			.serialize_with_options(&input, options)
			.unwrap();
		let text = std::str::from_utf8(&bytes).unwrap();
		// pretty JSON has newlines
		text.xref().xpect_contains("\n");
		let output: Pair = MediaType::Json.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[test]
	fn json_empty_bytes_null() {
		let result: () = MediaType::Json.deserialize(b"").unwrap();
		result.xpect_eq(());
	}

	#[test]
	fn roundtrip_ron() {
		let input = Pair { a: 7, b: 8 };
		let bytes = MediaType::Ron.serialize(&input).unwrap();
		let output: Pair = MediaType::Ron.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[test]
	fn ron_pretty() {
		let input = Pair { a: 7, b: 8 };
		let options = SerializeOptions { pretty: true };
		let bytes = MediaType::Ron
			.serialize_with_options(&input, options)
			.unwrap();
		let text = std::str::from_utf8(&bytes).unwrap();
		// pretty RON has newlines
		text.xref().xpect_contains("\n");
		let output: Pair = MediaType::Ron.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
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
	}

	#[cfg(feature = "json")]
	#[test]
	fn serialize_accepts_fallback_json() {
		let bytes = MediaType::serialize_accepts(&[], &42u32).unwrap();
		bytes.media_type().xpect_eq(MediaType::Json);
	}

	#[test]
	fn serialize_accepts_ron() {
		#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
		struct Val {
			num: u32,
		}
		let input = Val { num: 10 };
		let bytes =
			MediaType::serialize_accepts(&[MediaType::Ron], &input).unwrap();
		bytes.media_type().xpect_eq(MediaType::Ron);
		let output: Val = bytes.deserialize().unwrap();
		output.xpect_eq(input);
	}
}
