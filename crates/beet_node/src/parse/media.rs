//! Media-type-driven parser that dispatches to format-specific [`NodeParser`]
//! implementations based on the media type of [`ParseContext::bytes`].
//!
//! Enable additional parsers via feature flags:
//! - `html_parser` — adds [`HtmlParser`] support for [`MediaType::Html`]
//! - `markdown_parser` — adds [`MarkdownParser`] support for [`MediaType::Markdown`]

use crate::prelude::*;
use beet_core::prelude::*;

/// A [`NodeParser`] that selects the appropriate format-specific parser
/// based on [`ParseContext::bytes`]` media type.
///
/// Plain text is always available. HTML and markdown require their
/// respective feature flags. When `plaintext_fallback` is enabled,
/// any text-based media type without a dedicated parser falls back
/// to [`PlainTextParser`].
#[derive(Debug, Clone, Component)]
#[cfg_attr(feature = "net", component(on_add=on_add))]
pub struct MediaParser {
	/// Fall back to [`PlainTextParser`] for unrecognized text types.
	plaintext_fallback: bool,
	plain_text_parser: PlainTextParser,
	#[cfg(feature = "html_parser")]
	html_parser: HtmlParser,
	#[cfg(feature = "markdown_parser")]
	markdown_parser: MarkdownParser,
}

#[cfg(feature = "net")]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe(render_media);
}
#[cfg(feature = "net")]
fn render_media(ev: On<RenderMedia>, mut commands: Commands) -> Result {
	let entity = ev.event_target();
	let media_bytes = ev.event().clone();
	commands.queue(move |world: &mut World| -> Result {
		let mut entity = world.entity_mut(entity);
		// let mut parser = entity.get::<MediaParser>().unwrap().clone();
		let Some(mut parser) = entity.take::<MediaParser>() else {
			return Ok(());
		};

		// TODO this is a hack because our diffing is resulting in
		// stale data.. though we are visiting a new site, probs
		// fair to be clearing all anyway..
		entity.despawn_related::<Children>();

		parser.parse(ParseContext::new(&mut entity, &media_bytes))?;

		entity.insert(parser).trigger(NodeParsed);

		Ok(())
	});

	Ok(())
}




impl MediaParser {
	/// Create a parser with the plain-text fallback enabled.
	pub fn new() -> Self {
		Self {
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
}

impl Default for MediaParser {
	fn default() -> Self { Self::new() }
}

impl NodeParser for MediaParser {
	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		let media_type = cx.bytes.media_type().clone();
		match media_type {
			MediaType::Text => self.plain_text_parser.parse(cx),
			#[cfg(feature = "html_parser")]
			MediaType::Html => self.html_parser.parse(cx),
			#[cfg(feature = "markdown_parser")]
			MediaType::Markdown => self.markdown_parser.parse(cx),
			ref other if self.plaintext_fallback && other.is_text() => {
				self.plain_text_parser.parse(cx)
			}
			other => {
				let mut supported = vec![MediaType::Text];
				#[cfg(feature = "html_parser")]
				supported.push(MediaType::Html);
				#[cfg(feature = "markdown_parser")]
				supported.push(MediaType::Markdown);
				Err(ParseError::UnsupportedType {
					unsupported: other,
					supported,
				})
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
		let bytes = MediaBytes::new_text("hello");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MediaParser::new()
					.parse(ParseContext::new(entity, &bytes))
					.unwrap();
			})
			.child(0)
			.unwrap()
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("hello".into()));
	}

	#[cfg(feature = "html_parser")]
	#[test]
	fn parse_html() {
		let bytes = MediaBytes::new_html("<div>hello</div>");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MediaParser::new()
					.parse(ParseContext::new(entity, &bytes))
					.unwrap();
			})
			.children()
			.len()
			.xpect_eq(1);
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn parse_markdown() {
		let bytes = MediaBytes::new_markdown("# Title");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MediaParser::new()
					.parse(ParseContext::new(entity, &bytes))
					.unwrap();
			})
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
			.xpect_eq("h1".to_string());
	}

	#[test]
	fn fallback_text_type() {
		// CSS has no dedicated parser but is a text type,
		// so it should fall back to plain text.
		let bytes = MediaBytes::new_css("body { color: red; }");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				MediaParser::new()
					.parse(ParseContext::new(entity, &bytes))
					.unwrap();
			})
			.child(0)
			.unwrap()
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("body { color: red; }".into()));
	}

	#[test]
	fn no_fallback_errors() {
		let bytes = MediaBytes::new_css("body {}");
		MediaParser::new()
			.without_fallback()
			.parse(ParseContext::new(&mut World::new().spawn_empty(), &bytes))
			.xpect_err();
	}

	#[test]
	fn binary_type_errors() {
		let bytes = MediaBytes::new(MediaType::Png, b"\x89PNG".as_slice());
		MediaParser::new()
			.parse(ParseContext::new(&mut World::new().spawn_empty(), &bytes))
			.xpect_err();
	}
}
