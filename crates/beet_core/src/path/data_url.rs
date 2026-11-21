//! [`DataUrl`], bytes carried in a `data:` URL.
use crate::prelude::*;
use base64::Engine;

/// Bytes and their media type carried in a `data:` URL, as RFC 2397 spells
/// one, ie `data:image/png;base64,iVBORw0K..`: the one way a document holds
/// what it shows without a reference elsewhere, so reading or writing one
/// does no IO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataUrl {
	/// The media type, ie `image/png`, `text/plain;charset=US-ASCII` where
	/// the URL names none.
	pub media_type: SmolStr,
	/// The bytes.
	pub bytes: Vec<u8>,
}

impl DataUrl {
	/// The media type a URL naming none carries.
	pub const DEFAULT_TYPE: &str = "text/plain;charset=US-ASCII";

	/// `bytes` of `media_type`.
	pub fn new(
		media_type: impl Into<SmolStr>,
		bytes: impl Into<Vec<u8>>,
	) -> Self {
		Self {
			media_type: media_type.into(),
			bytes: bytes.into(),
		}
	}

	/// Whether `url` is a `data:` URL, its scheme matched case-insensitively.
	pub fn is_data_url(url: &str) -> bool {
		url.get(..5)
			.is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:"))
	}

	/// Parses a `data:` URL, its payload base64 when it says so and
	/// percent-encoded otherwise.
	pub fn parse(url: &str) -> Result<Self> {
		if !Self::is_data_url(url) {
			bevybail!("`{}` is no `data:` URL", Self::abbreviate(url));
		}
		let Some((header, payload)) = url[5..].split_once(',') else {
			bevybail!("the `data:` URL `{}` has no `,`", Self::abbreviate(url));
		};
		let (media_type, base64) = match header.strip_suffix(";base64") {
			Some(media_type) => (media_type, true),
			None => (header, false),
		};
		let bytes = match base64 {
			// a payload may be wrapped, so whitespace is dropped; a no_std
			// decode error is no `Error`, so it is named here
			true => base64::engine::general_purpose::STANDARD
				.decode(
					payload
						.bytes()
						.filter(|byte| !byte.is_ascii_whitespace())
						.collect::<Vec<_>>(),
				)
				.map_err(|err| {
					bevyhow!(
						"the `data:` URL `{}` holds malformed base64: {err}",
						Self::abbreviate(url)
					)
				})?,
			false => Self::percent_decode(payload),
		};
		Self {
			media_type: match media_type.is_empty() {
				true => Self::DEFAULT_TYPE.into(),
				false => media_type.into(),
			},
			bytes,
		}
		.xok()
	}

	/// The media type without its parameters, ie `image/png`.
	pub fn essence(&self) -> &str {
		self.media_type.split(';').next().unwrap_or_default().trim()
	}

	/// The first characters of `url`, for an error naming it.
	fn abbreviate(url: &str) -> String {
		match url.char_indices().nth(40) {
			Some((index, _)) => format!("{}..", &url[..index]),
			None => url.to_string(),
		}
	}

	/// `payload` with every `%XX` escape decoded.
	fn percent_decode(payload: &str) -> Vec<u8> {
		let bytes = payload.as_bytes();
		let mut out = Vec::with_capacity(bytes.len());
		let mut index = 0;
		while index < bytes.len() {
			let hex = bytes
				.get(index + 1..index + 3)
				.and_then(|hex| core::str::from_utf8(hex).ok())
				.and_then(|hex| u8::from_str_radix(hex, 16).ok());
			match (bytes[index], hex) {
				(b'%', Some(byte)) => {
					out.push(byte);
					index += 3;
				}
				(byte, _) => {
					out.push(byte);
					index += 1;
				}
			}
		}
		out
	}
}

impl core::fmt::Display for DataUrl {
	/// The URL, its payload base64.
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"data:{};base64,{}",
			self.media_type,
			base64::engine::general_purpose::STANDARD.encode(&self.bytes)
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn round_trips() {
		let url = DataUrl::new("image/png", vec![0x89, b'P', b'N', b'G']);
		let text = url.to_string();
		text.clone().xpect_eq("data:image/png;base64,iVBORw==");
		DataUrl::parse(&text).unwrap().xpect_eq(url);
	}

	#[crate::test]
	fn parses_plain_and_defaulted_payloads() {
		let plain = DataUrl::parse("DATA:,a%20b").unwrap();
		plain.media_type.clone().xpect_eq(DataUrl::DEFAULT_TYPE);
		plain.bytes.xpect_eq(b"a b".to_vec());
		DataUrl::parse("data:image/svg+xml;charset=utf-8;base64,PHN2Zy8+")
			.unwrap()
			.essence()
			.xpect_eq("image/svg+xml");
		DataUrl::parse("https://example.com").xpect_err();
		DataUrl::parse("data:image/png;base64").xpect_err();
	}
}
