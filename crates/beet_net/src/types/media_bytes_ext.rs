//! URL ↔ [`MediaBytes`] conversion for `data:` URIs.

use beet_core::prelude::*;

use crate::prelude::*;

/// Extends [`MediaBytes`] with `data:` URI construction and parsing.
#[extend::ext(name = MediaBytesUrlExt)]
pub impl MediaBytes {
	/// Parse a `data:` [`Url`] directly into [`MediaBytes`], decoding the
	/// payload in a single step.
	///
	/// Supports `base64` and percent-encoded (URL-encoded) payloads.
	///
	/// ## Errors
	///
	/// - If `url` does not use the `data:` scheme.
	/// - If the payload is missing a `,` separator.
	/// - If base64 decoding fails (requires the `serde` feature).
	///
	/// # Example
	///
	/// ```
	/// # use beet_net::prelude::*;
	/// # use beet_core::prelude::*;
	/// let url = Url::parse("data:text/plain;base64,SGVsbG8=");
	/// let mb = MediaBytes::from_url(&url).unwrap();
	/// assert_eq!(mb.media_type(), &MediaType::Text);
	/// assert_eq!(mb.as_utf8().unwrap(), "Hello");
	/// ```
	fn from_url(url: &Url) -> Result<MediaBytes> {
		if url.scheme() != &Scheme::Data {
			bevybail!("expected a data: URL, got scheme '{}'", url.scheme());
		}

		let payload = url.path().first().map(|seg| seg.as_str()).unwrap_or("");

		let comma = payload.find(',').ok_or_else(|| {
			bevyhow!("data URI missing ',' separator: {payload:?}")
		})?;

		let header = &payload[..comma];
		let data = &payload[comma + 1..];

		// Determine encoding and strip the token from the header.
		let (media_str, is_base64) =
			if header.ends_with(";base64") || header == ";base64" {
				let media_str = header
					.strip_suffix(";base64")
					.unwrap_or("")
					.trim_start_matches(';');
				(media_str, true)
			} else {
				(header.trim_start_matches(';'), false)
			};

		let media_type = if media_str.is_empty() {
			MediaType::Text
		} else {
			MediaType::from_content_type(media_str)
		};

		let bytes: Vec<u8> = if is_base64 {
			cfg_if! {
				if #[cfg(feature = "serde")] {
					use base64::Engine as _;
					base64::engine::general_purpose::STANDARD
						.decode(data)
						.map_err(|err| bevyhow!("base64 decode failed: {err}"))?
				} else {
					bevybail!("base64 decoding requires the 'serde' feature");
				}
			}
		} else {
			// Percent-decode
			percent_decode(data)
		};

		Ok(MediaBytes::new(media_type, bytes))
	}

	/// Encode `self` as a `data:` [`Url`] using base64.
	///
	/// # Example
	///
	/// ```
	/// # use beet_net::prelude::*;
	/// # use beet_core::prelude::*;
	/// let mb = MediaBytes::new_text("Hello");
	/// let url = mb.into_url();
	/// assert_eq!(url.scheme(), &Scheme::Data);
	/// // Round-trip
	/// let back = MediaBytes::from_url(&url).unwrap();
	/// assert_eq!(back.as_utf8().unwrap(), "Hello");
	/// ```
	fn into_url(&self) -> Url {
		cfg_if! {
			if #[cfg(feature = "serde")] {
				use base64::Engine as _;
				let encoded =
					base64::engine::general_purpose::STANDARD.encode(self.bytes());
				Url::parse(format!("data:{};base64,{}", self.media_type(), encoded))
			} else {
				// Fall back to URL (percent) encoding when base64 is unavailable.
				let encoded = percent_encode(self.bytes());
				Url::parse(format!("data:{},{}", self.media_type(), encoded))
			}
		}
	}
}

/// Percent-decode a data URI payload string into bytes.
fn percent_decode(input: &str) -> Vec<u8> {
	input
		.split('%')
		.enumerate()
		.fold(String::new(), |mut acc, (i, chunk)| {
			if i == 0 {
				acc.push_str(chunk);
			} else if chunk.len() >= 2 {
				if let Ok(byte) = u8::from_str_radix(&chunk[..2], 16) {
					acc.push(byte as char);
					acc.push_str(&chunk[2..]);
				} else {
					acc.push('%');
					acc.push_str(chunk);
				}
			} else {
				acc.push('%');
				acc.push_str(chunk);
			}
			acc
		})
		.into_bytes()
}

/// Percent-encode raw bytes for use in a URL-encoded data URI.
#[cfg(not(feature = "serde"))]
fn percent_encode(bytes: &[u8]) -> String {
	bytes.iter().fold(String::new(), |mut acc, &byte| {
		// Safe unreserved characters per RFC 3986
		if byte.is_ascii_alphanumeric()
			|| byte == b'-'
			|| byte == b'_'
			|| byte == b'.'
			|| byte == b'~'
		{
			acc.push(byte as char);
		} else {
			acc.push_str(&format!("%{:02X}", byte));
		}
		acc
	})
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn from_url_base64() {
		let url = Url::parse("data:text/plain;base64,SGVsbG8=");
		let mb = MediaBytes::from_url(&url).unwrap();
		mb.media_type().xpect_eq(MediaType::Text);
		mb.as_utf8().unwrap().xpect_eq("Hello");
	}

	#[test]
	fn from_url_percent_encoded() {
		let url = Url::parse("data:text/html,<h1>Hello</h1>");
		let mb = MediaBytes::from_url(&url).unwrap();
		mb.media_type().xpect_eq(MediaType::Html);
		mb.as_utf8().unwrap().xpect_eq("<h1>Hello</h1>");
	}

	#[test]
	fn from_url_default_media_type() {
		// RFC 2397: missing media type defaults to text/plain
		let url = Url::parse("data:,Hello%20World");
		let mb = MediaBytes::from_url(&url).unwrap();
		mb.media_type().xpect_eq(MediaType::Text);
		// %20 → space
		mb.as_utf8().unwrap().xpect_eq("Hello World");
	}

	#[test]
	fn from_url_wrong_scheme() {
		let url = Url::parse("https://example.com");
		MediaBytes::from_url(&url).xpect_err();
	}

	#[cfg(feature = "serde")]
	#[test]
	fn into_url_base64() {
		let mb = MediaBytes::new_text("Hello");
		let url = mb.into_url();
		url.scheme().xpect_eq(Scheme::Data);
		// The payload segment should contain the base64 token.
		let segment = url.path().first().unwrap();
		segment.xpect_contains(";base64,");
	}

	#[cfg(feature = "serde")]
	#[test]
	fn round_trip() {
		let original = MediaBytes::new_html("<p>hi</p>");
		let url = original.into_url();
		let back = MediaBytes::from_url(&url).unwrap();
		back.media_type().xpect_eq(MediaType::Html);
		back.as_utf8().unwrap().xpect_eq("<p>hi</p>");
	}

	#[cfg(feature = "serde")]
	#[test]
	fn round_trip_display_reparse() {
		let mb = MediaBytes::new_text("Hello");
		let url = mb.into_url();
		// Display then re-parse should produce identical bytes.
		let reparsed = Url::parse(url.to_string());
		let back = MediaBytes::from_url(&reparsed).unwrap();
		back.as_utf8().unwrap().xpect_eq("Hello");
	}
}
