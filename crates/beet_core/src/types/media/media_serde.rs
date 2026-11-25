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
//!
//! Each format rides its own feature (`json`, `toml`, `postcard`,
//! `serde_plain`, `dag_cbor`), and a build without it answers the same
//! "feature is required" error from every caller, so a disabled format fails
//! identically everywhere. Why RON is none of them is written once, in
//! `beet_scene`'s `StructuredParser` docs. [`MediaType::Yaml`] is beet's own, the subset a
//! frontmatter block uses read and written through [`Value`], so it needs no
//! feature beyond `serde` (see the `yaml` module). [`MediaType::DagCbor`] is canonical DAG-CBOR
//! through the atproto data model ([`AtprotoValue`]): a link is CBOR tag 42,
//! bytes a byte string and a float the object it crosses as, so its output is
//! exactly the block a PDS stores.
use crate::prelude::*;

/// Options for controlling serialization output.
#[derive(Debug, Clone, Default)]
pub struct SerializeOptions {
	/// Serialize in a pretty format where possible.
	pub pretty: bool,
}

impl MediaType {
	/// [`deserialize`](Self::deserialize) a document, its error a
	/// [`SourceError`] where the format's reader says it is: json's and
	/// TOML's do.
	pub fn deserialize_document<T: serde::de::DeserializeOwned>(
		&self,
		bytes: &[u8],
	) -> Result<T> {
		self.deserialize::<T>(bytes).map_err(|error| {
			match self.error_place::<T>(bytes) {
				Some((offset, message)) => {
					SourceError::at(bevyhow!("{message}"), offset)
				}
				None => error,
			}
		})
	}

	/// Where `bytes` fail to read as a `T` and why, less the line and column
	/// the reader counts in them, where the format's reader says.
	fn error_place<T: serde::de::DeserializeOwned>(
		&self,
		bytes: &[u8],
	) -> Option<(usize, String)> {
		match self {
			#[cfg(feature = "json")]
			MediaType::Json => {
				let error = serde_json::from_slice::<T>(bytes).err()?;
				// its line is zero for an error of no place
				let line = error.line().checked_sub(1)?;
				let start = bytes
					.split_inclusive(|byte| *byte == b'\n')
					.take(line)
					.map(<[u8]>::len)
					.sum::<usize>();
				let message = error.to_string();
				let message = message
					.rsplit_once(" at line ")
					.map_or(message.as_str(), |(message, _)| message);
				Some((
					start + error.column().saturating_sub(1),
					message.to_string(),
				))
			}
			#[cfg(feature = "toml")]
			MediaType::Toml => {
				let text = core::str::from_utf8(bytes).ok()?;
				let error = toml::from_str::<T>(text).err()?;
				Some((error.span()?.start, error.message().to_string()))
			}
			_ => {
				let _ = bytes;
				None
			}
		}
	}

	/// The structured data formats this build reads, each behind its own
	/// feature: json, postcard and toml.
	pub fn structured() -> Vec<MediaType> {
		[
			cfg!(feature = "json").then_some(MediaType::Json),
			cfg!(feature = "postcard").then_some(MediaType::Postcard),
			cfg!(feature = "toml").then_some(MediaType::Toml),
		]
		.into_iter()
		.flatten()
		.collect()
	}

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
		if accept.is_empty() || MediaType::any_wildcard(accept) {
			cfg_if! {
				if #[cfg(feature = "json")]{
					let value = if options.pretty {
						serde_json::to_string_pretty(value)?
					} else {
						serde_json::to_string(value)?
					};
					Ok(MediaBytes::new(MediaType::Json, value.into_bytes()))
				} else if #[cfg(feature = "serde_plain")] {
					let value = serde_plain::to_string(value)?;
					Ok(MediaBytes::new(MediaType::Text, value.into_bytes()))
				} else {
					let _ = (value, options);
					bevybail!(
						"No fallback serialization format; enable the `json` or `serde_plain` feature"
					)
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
	/// | `Text`               | plain    | `serde_plain`   |
	/// | `Json`               | JSON     | `json`          |
	/// | `Toml`               | TOML     | `toml`          |
	/// | `Yaml`               | YAML, frontmatter's subset | `serde` |
	/// | `DagCbor`            | DAG-CBOR | `dag_cbor`      |
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
				cfg_if! {
					if #[cfg(feature = "serde_plain")] {
						let value = serde_plain::to_string(value)?;
						Ok(value.into_bytes())
					} else {
						let _ = value;
						bevybail!(
							"The `serde_plain` feature is required for plaintext serialization"
						)
					}
				}
			}
			MediaType::Json => {
				cfg_if! {
					if #[cfg(feature = "json")] {
						if options.pretty {
							serde_json::to_vec_pretty(value).map_err(|err| {
								bevyhow!("Failed to serialize JSON: {err}")
							})
						} else {
							serde_json::to_vec(value).map_err(|err| {
								bevyhow!("Failed to serialize JSON: {err}")
							})
						}
					} else {
						let _ = (value, options);
						bevybail!(
							"The `json` feature is required for JSON serialization"
						)
					}
				}
			}
			MediaType::Toml => {
				cfg_if! {
					if #[cfg(feature = "toml")] {
						// tables after scalars either way; pretty spreads an
						// array over a line per item
						match options.pretty {
							true => toml::to_string_pretty(value),
							false => toml::to_string(value),
						}
						.map(|text| text.into_bytes())
						.map_err(|err| {
							bevyhow!("Failed to serialize TOML: {err}")
						})
					} else {
						let _ = (value, options);
						bevybail!(
							"The `toml` feature is required for TOML serialization"
						)
					}
				}
			}
			// one layout, through the data model
			MediaType::Yaml => {
				let _ = options;
				super::yaml::Yaml::write(&Value::from_serde(value)?)
					.map(String::into_bytes)
			}
			MediaType::DagCbor => {
				cfg_if! {
					if #[cfg(feature = "dag_cbor")] {
						// one canonical layout, through the data model
						let _ = options;
						AtprotoValue::from_serde(value)?.to_dag_cbor().xok()
					} else {
						let _ = (value, options);
						bevybail!(
							"The `dag_cbor` feature is required for DAG-CBOR serialization"
						)
					}
				}
			}
			MediaType::Postcard | MediaType::Bytes => {
				cfg_if! {
					if #[cfg(feature = "postcard")] {
						let _ = options;
						postcard::to_allocvec(value).map_err(|err| {
							bevyhow!("Failed to serialize postcard: {err}")
						})
					} else {
						let _ = (value, options);
						bevybail!(
							"The `postcard` feature is required for postcard serialization"
						)
					}
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
	/// [`MediaType::DagCbor`] crosses through the atproto data model: a link
	/// is CBOR tag 42, bytes a byte string, and a float the
	/// `org.beet.core#float` object it was written as (see
	/// [`AtprotoValue`]), so serializing is exactly the block a PDS stores.
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
	/// | `Text`               | plain    | `serde_plain`   |
	/// | `Json`               | JSON     | `json`          |
	/// | `Toml`               | TOML     | `toml`          |
	/// | `Yaml`               | YAML, frontmatter's subset | `serde` |
	/// | `DagCbor`            | DAG-CBOR | `dag_cbor`      |
	/// | `Postcard` / `Bytes` | postcard | `postcard`      |
	#[cfg(feature = "serde")]
	pub fn deserialize<T: serde::de::DeserializeOwned>(
		&self,
		bytes: &[u8],
	) -> Result<T> {
		match self {
			MediaType::Text => {
				cfg_if! {
					if #[cfg(feature = "serde_plain")] {
						let string = core::str::from_utf8(bytes)?;
						serde_plain::from_str(string).map_err(|err| {
							bevyhow!("Failed to deserialize plaintext body: {err}")
						})
					} else {
						let _ = bytes;
						bevybail!(
							"The `serde_plain` feature is required for plaintext deserialization"
						)
					}
				}
			}
			MediaType::Json => {
				cfg_if! {
					if #[cfg(feature = "json")] {
						let slice = if bytes.is_empty() { b"null" } else { bytes };
						serde_json::from_slice(slice).map_err(|err| {
							bevyhow!("Failed to deserialize JSON body: {err}")
						})
					} else {
						let _ = bytes;
						bevybail!(
							"The `json` feature is required for JSON deserialization"
						)
					}
				}
			}
			MediaType::Toml => {
				cfg_if! {
					if #[cfg(feature = "toml")] {
						let string = core::str::from_utf8(bytes).map_err(|err| {
							bevyhow!("TOML data is not valid UTF-8: {err}")
						})?;
						toml::from_str(string).map_err(|err| {
							bevyhow!("Failed to deserialize TOML body: {err}")
						})
					} else {
						let _ = bytes;
						bevybail!(
							"The `toml` feature is required for TOML deserialization"
						)
					}
				}
			}
			MediaType::Yaml => {
				super::yaml::Yaml::read(core::str::from_utf8(bytes)?)?
					.into_serde()
			}
			MediaType::DagCbor => {
				cfg_if! {
					if #[cfg(feature = "dag_cbor")] {
						AtprotoValue::from_dag_cbor(bytes)?.into_serde()
					} else {
						let _ = bytes;
						bevybail!(
							"The `dag_cbor` feature is required for DAG-CBOR deserialization"
						)
					}
				}
			}
			MediaType::Postcard | MediaType::Bytes => {
				cfg_if! {
					if #[cfg(feature = "postcard")] {
						postcard::from_bytes(bytes).map_err(|err| {
							bevyhow!("Failed to deserialize postcard body: {err}")
						})
					} else {
						let _ = bytes;
						bevybail!(
							"The `postcard` feature is required for postcard deserialization"
						)
					}
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

	#[cfg(feature = "serde_plain")]
	#[crate::test]
	fn roundtrip_plaintext() {
		let input: u32 = 20;
		let bytes = MediaType::Text.serialize(&input).unwrap();
		let output: u32 = MediaType::Text.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[crate::test]
	fn roundtrip_json() {
		let input = Pair { a: 1, b: 2 };
		let bytes = MediaType::Json.serialize(&input).unwrap();
		let output: Pair = MediaType::Json.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[crate::test]
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
	#[crate::test]
	fn json_empty_bytes_null() {
		let result: () = MediaType::Json.deserialize(b"").unwrap();
		result.xpect_eq(());
	}

	#[cfg(feature = "postcard")]
	#[crate::test]
	fn roundtrip_postcard() {
		let input = Pair { a: 3, b: 4 };
		let bytes = MediaType::Postcard.serialize(&input).unwrap();
		let output: Pair = MediaType::Postcard.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "postcard")]
	#[crate::test]
	fn bytes_media_type_uses_postcard() {
		let input = Pair { a: 5, b: 6 };
		let bytes = MediaType::Bytes.serialize(&input).unwrap();
		let output: Pair = MediaType::Bytes.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[cfg(feature = "json")]
	#[crate::test]
	fn unsupported_media_type_errors() {
		MediaType::Html.serialize(&42u32).xpect_err();
	}

	#[cfg(feature = "json")]
	#[crate::test]
	fn serialize_accepts_fallback_json() {
		let bytes = MediaType::serialize_accepts(&[], &42u32).unwrap();
		bytes.media_type().xpect_eq(MediaType::Json);
	}

	#[cfg(feature = "toml")]
	#[crate::test]
	fn serialize_accepts_toml() {
		#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
		struct Val {
			num: u32,
		}
		let input = Val { num: 10 };
		let bytes =
			MediaType::serialize_accepts(&[MediaType::Toml], &input).unwrap();
		bytes.media_type().xpect_eq(MediaType::Toml);
		let output: Val = bytes.deserialize().unwrap();
		output.xpect_eq(input);
	}
}
