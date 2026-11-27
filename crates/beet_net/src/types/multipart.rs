//! A `multipart/mixed` body (RFC 2046), see [`Multipart`].
use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// Several bodies in one, each part its own `Content-Type` and, for a file,
/// a `Content-Disposition` filename: an answer of several files, ie a tool
/// reading three posts' images, is one `multipart/mixed` body.
///
/// ```
/// # use beet_net::prelude::*;
/// # use beet_core::prelude::*;
/// let multipart = Multipart::default()
/// 	.with_part(MediaBytes::new_text("two files").into())
/// 	.with_part(MultipartPart::new(MediaBytes::new(MediaType::Png, vec![1, 2])).with_filename("cat.png"));
/// let (content_type, body) = multipart.encode();
/// Multipart::parse(&content_type, &body).unwrap().xpect_eq(multipart);
/// ```
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Multipart {
	parts: Vec<MultipartPart>,
}

/// One body of a [`Multipart`]: its bytes and their `Content-Type`, and for
/// a file its `Content-Disposition` filename.
#[derive(Debug, Clone, PartialEq)]
pub struct MultipartPart {
	media: MediaBytes,
	/// The part's `Content-Disposition` filename, for a file.
	filename: Option<SmolStr>,
}

impl MultipartPart {
	/// A part holding `media`.
	pub fn new(media: impl Into<MediaBytes>) -> Self {
		Self {
			media: media.into(),
			filename: None,
		}
	}

	/// This part as a file named `filename`.
	pub fn with_filename(mut self, filename: impl Into<SmolStr>) -> Self {
		self.filename = Some(filename.into());
		self
	}

	/// The part's bytes and their media type.
	pub fn media(&self) -> &MediaBytes { &self.media }
	/// The part's filename, for a file.
	pub fn filename(&self) -> Option<&str> { self.filename.as_deref() }
	/// The name without its extension, ie `cat` of `cat.png`.
	pub fn file_stem(&self) -> Option<&str> {
		self.filename()
			.map(|name| name.rsplit_once('.').map_or(name, |(stem, _)| stem))
	}
}

impl From<MediaBytes> for MultipartPart {
	fn from(media: MediaBytes) -> Self { Self::new(media) }
}

impl From<MultipartPart> for MediaBytes {
	fn from(part: MultipartPart) -> Self { part.media }
}

impl Multipart {
	/// This multipart with `part` appended.
	pub fn with_part(mut self, part: MultipartPart) -> Self {
		self.parts.push(part);
		self
	}

	/// Append `part`.
	pub fn push(&mut self, part: MultipartPart) { self.parts.push(part); }

	/// The parts, in order.
	pub fn parts(&self) -> &[MultipartPart] { &self.parts }
	/// The parts, owned.
	pub fn into_parts(self) -> Vec<MultipartPart> { self.parts }
	/// Whether it holds no part.
	pub fn is_empty(&self) -> bool { self.parts.is_empty() }

	/// The body, and the `Content-Type` naming its boundary, ie
	/// `multipart/mixed; boundary=beet-part-0`. The boundary is the first of
	/// `beet-part-<n>` no part holds.
	pub fn encode(&self) -> (String, Vec<u8>) {
		let boundary = (0..)
			.map(|index| format!("beet-part-{index}"))
			.find(|boundary| {
				!self.parts.iter().any(|part| {
					part.media
						.bytes()
						.windows(boundary.len())
						.any(|window| window == boundary.as_bytes())
				})
			})
			.unwrap_or_default();
		let mut body = Vec::new();
		for part in &self.parts {
			body.extend_from_slice(
				format!(
					"--{boundary}\r\nContent-Type: {}\r\n",
					part.media.media_type()
				)
				.as_bytes(),
			);
			if let Some(filename) = &part.filename {
				body.extend_from_slice(
					format!(
						"Content-Disposition: attachment; filename=\"{filename}\"\r\n"
					)
					.as_bytes(),
				);
			}
			body.extend_from_slice(b"\r\n");
			body.extend_from_slice(part.media.bytes());
			body.extend_from_slice(b"\r\n");
		}
		body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
		(
			format!("{}; boundary={boundary}", MediaType::MultipartMixed),
			body,
		)
	}

	/// Read `body` by the boundary its `content_type` names.
	///
	/// ## Errors
	///
	/// Errors when `content_type` names no boundary, or a part has no blank
	/// line ending its headers.
	pub fn parse(content_type: &str, body: &[u8]) -> Result<Self> {
		let boundary = content_type
			.split(';')
			.filter_map(|param| param.trim().strip_prefix("boundary="))
			.next()
			.map(|boundary| boundary.trim_matches('"'))
			.ok_or_else(|| {
				bevyhow!("the content type `{content_type}` names no boundary")
			})?;
		let delimiter = format!("--{boundary}");
		let mut parts = Vec::new();
		// each segment after a delimiter is a part, until the closing `--`
		for segment in split_bytes(body, delimiter.as_bytes()).skip(1) {
			if segment.starts_with(b"--") {
				break;
			}
			let segment = segment.strip_prefix(b"\r\n").unwrap_or(segment);
			let end = find_bytes(segment, b"\r\n\r\n").ok_or_else(|| {
				bevyhow!(
					"a part of the multipart body has no blank line ending its headers"
				)
			})?;
			let headers = String::from_utf8_lossy(&segment[..end]);
			let bytes = &segment[end + 4..];
			let bytes = bytes.strip_suffix(b"\r\n").unwrap_or(bytes);
			let mut media_type = MediaType::Text;
			let mut filename = None;
			for line in headers.lines() {
				let Some((name, value)) = line.split_once(':') else {
					continue;
				};
				match name.trim().to_ascii_lowercase().as_str() {
					"content-type" => {
						media_type = MediaType::from_content_type(value.trim());
					}
					"content-disposition" => {
						filename = value
							.split(';')
							.filter_map(|param| {
								param.trim().strip_prefix("filename=")
							})
							.next()
							.map(|name| SmolStr::from(name.trim_matches('"')));
					}
					_ => {}
				}
			}
			parts.push(MultipartPart {
				media: MediaBytes::new(
					media_type,
					Bytes::copy_from_slice(bytes),
				),
				filename,
			});
		}
		Self { parts }.xok()
	}

	/// A `200` answering this multipart.
	pub fn into_response(self) -> Response {
		let (content_type, body) = self.encode();
		Response::ok()
			.with_header("content-type", &content_type)
			.with_body(body)
	}

	/// The multipart `response` answers, `None` when its body is not
	/// `multipart/mixed`, handing the response back.
	///
	/// ## Errors
	///
	/// Errors when its body cannot be read or parsed.
	pub async fn from_response(
		response: Response,
	) -> Result<core::result::Result<Self, Response>> {
		let content_type = response
			.response_parts()
			.headers
			.first_raw("content-type")
			.map(str::to_string);
		match content_type {
			Some(content_type)
				if MediaType::from_content_type(&content_type)
					== MediaType::MultipartMixed =>
			{
				let (_, body) = response.into_parts();
				Self::parse(&content_type, &body.into_bytes().await?)?
					.xok()
					.xok()
			}
			_ => Ok(Err(response)),
		}
	}
}

/// The position of the first `needle` in `haystack`.
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
	haystack
		.windows(needle.len())
		.position(|window| window == needle)
}

/// `haystack` split on each `needle`.
fn split_bytes<'a>(
	mut haystack: &'a [u8],
	needle: &'a [u8],
) -> impl Iterator<Item = &'a [u8]> {
	let mut done = false;
	core::iter::from_fn(move || {
		if done {
			return None;
		}
		match find_bytes(haystack, needle) {
			Some(index) => {
				let segment = &haystack[..index];
				haystack = &haystack[index + needle.len()..];
				Some(segment)
			}
			None => {
				done = true;
				Some(haystack)
			}
		}
	})
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A part holding the first boundary is encoded under the next, and a
	/// response carries the multipart whole.
	#[beet_core::test]
	async fn round_trips_through_a_response() {
		let multipart = Multipart::default()
			.with_part(MediaBytes::new_text("--beet-part-0").into())
			.with_part(
				MultipartPart::new(MediaBytes::new(MediaType::Png, vec![
					0, 13, 10, 255,
				]))
				.with_filename("cat.png"),
			);
		multipart
			.encode()
			.0
			.xpect_eq("multipart/mixed; boundary=beet-part-1");
		let read = Multipart::from_response(multipart.clone().into_response())
			.await
			.unwrap()
			.unwrap();
		read.parts()[1].file_stem().xpect_eq(Some("cat"));
		read.xpect_eq(multipart);
		// any other body is handed back
		Multipart::from_response(Response::ok_text("hi"))
			.await
			.unwrap()
			.unwrap_err()
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	fn refuses_a_missing_boundary() {
		Multipart::parse("multipart/mixed", b"")
			.unwrap_err()
			.to_string()
			.xpect_contains("names no boundary");
	}
}
