//! Media-type-driven parser that dispatches to format-specific [`NodeParser`]
//! implementations based on a [`MediaType`] value.
//!
//! Enable additional parsers via feature flags:
//! - `html_parser` — adds [`HtmlParser`] support for [`MediaType::Html`]
//! - `markdown_parser` — adds [`MarkdownParser`] support for [`MediaType::Markdown`]

use crate::prelude::*;
use beet_core::prelude::*;

/// A [`NodeParser`] that selects the appropriate format-specific parser
/// based on a [`MediaType`].
///
/// Plain text is always available. HTML and markdown require their
/// respective feature flags. When `plaintext_fallback` is enabled,
/// any text-based media type without a dedicated parser falls back
/// to [`PlainTextParser`].
pub struct MediaParser {
	/// The media type to parse as.
	media_type: MediaType,
	/// Fall back to [`PlainTextParser`] for unrecognized text types.
	plaintext_fallback: bool,
	plain_text_parser: PlainTextParser,
	#[cfg(feature = "html_parser")]
	html_parser: HtmlParser,
	#[cfg(feature = "markdown_parser")]
	markdown_parser: MarkdownParser,
}

impl MediaParser {
	/// Create a parser targeting the given media type.
	pub fn new(media_type: MediaType) -> Self {
		Self {
			media_type,
			plaintext_fallback: true,
			plain_text_parser: PlainTextParser::default(),
			#[cfg(feature = "html_parser")]
			html_parser: HtmlParser::new(),
			#[cfg(feature = "markdown_parser")]
			markdown_parser: MarkdownParser::new(),
		}
	}

	/// Disable the plain-text fallback for unrecognized text types.
	pub fn without_fallback(mut self) -> Self {
		self.plaintext_fallback = false;
		self
	}

	/// Override the [`PlainTextParser`] instance.
	pub fn with_plain_text_parser(mut self, parser: PlainTextParser) -> Self {
		self.plain_text_parser = parser;
		self
	}

	/// Override the [`HtmlParser`] instance.
	#[cfg(feature = "html_parser")]
	pub fn with_html_parser(mut self, parser: HtmlParser) -> Self {
		self.html_parser = parser;
		self
	}

	/// Override the [`MarkdownParser`] instance.
	#[cfg(feature = "markdown_parser")]
	pub fn with_markdown_parser(mut self, parser: MarkdownParser) -> Self {
		self.markdown_parser = parser;
		self
	}

	/// Change the target media type.
	pub fn with_media_type(mut self, media_type: MediaType) -> Self {
		self.media_type = media_type;
		self
	}

	/// Change the target media type.
	pub fn set_media_type(&mut self, media_type: MediaType) {
		self.media_type = media_type;
	}

	/// Returns the currently configured media type.
	pub fn media_type(&self) -> &MediaType { &self.media_type }
}

impl NodeParser for MediaParser {
	fn parse(
		&mut self,
		entity: &mut EntityWorldMut,
		bytes: &[u8],
		path: Option<WsPathBuf>,
	) -> Result {
		match self.media_type {
			MediaType::Text => {
				self.plain_text_parser.parse(entity, bytes, path)
			}
			#[cfg(feature = "html_parser")]
			MediaType::Html => self.html_parser.parse(entity, bytes, path),
			#[cfg(feature = "markdown_parser")]
			MediaType::Markdown => self.markdown_parser.parse(entity, bytes, path),
			ref other if self.plaintext_fallback && other.is_text() => {
				self.plain_text_parser.parse(entity, bytes, path)
			}
			ref other => {
				bevybail!("no parser available for media type: {other}")
			}
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn parse_plain_text() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MediaParser::new(MediaType::Text)
					.parse(entity, b"hello", None)
					.unwrap();
			})
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("hello".into()));
	}

	#[cfg(feature = "html_parser")]
	#[test]
	fn parse_html() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MediaParser::new(MediaType::Html)
					.parse(entity, b"<div>hello</div>", None)
					.unwrap();
			})
			.children()
			.len()
			.xpect_eq(1);
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn parse_markdown() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MediaParser::new(MediaType::Markdown)
					.parse(entity, b"# Title", None)
					.unwrap();
			})
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.name()
			.to_string()
			.xpect_eq("h1".to_string());
	}

	#[test]
	fn fallback_text_type() {
		// CSS has no dedicated parser but is a text type,
		// so it should fall back to plain text.
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MediaParser::new(MediaType::Css)
					.parse(entity, b"body { color: red; }", None)
					.unwrap();
			})
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("body { color: red; }".into()));
	}

	#[test]
	fn no_fallback_errors() {
		MediaParser::new(MediaType::Css)
			.without_fallback()
			.parse(&mut World::new().spawn_empty(), b"body {}", None)
			.xpect_err();
	}

	#[test]
	fn binary_type_errors() {
		MediaParser::new(MediaType::Png)
			.parse(&mut World::new().spawn_empty(), b"\x89PNG", None)
			.xpect_err();
	}
}
