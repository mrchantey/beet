//! Media-type-driven renderer that dispatches to format-specific
//! [`NodeRenderer`] implementations based on a [`MediaType`] value.
//!
//! Enable additional renderers via feature flags:
//! - `ansi_term` — adds [`AnsiTermRenderer`] support

use crate::prelude::*;
use beet_core::prelude::*;

/// A [`NodeRenderer`] that selects the appropriate format-specific renderer
/// based on a [`MediaType`].
///
/// Plain text, HTML, and markdown renderers are always available.
/// The ANSI terminal renderer requires the `ansi_term` feature flag.
/// When `plaintext_fallback` is enabled, any text-based media type
/// without a dedicated renderer falls back to [`PlainTextRenderer`].
pub struct MediaRenderer {
	/// The media type to render as.
	media_type: MediaType,
	/// Fall back to [`PlainTextRenderer`] for unrecognized text types.
	plaintext_fallback: bool,
	plain_text_renderer: PlainTextRenderer,
	html_renderer: HtmlRenderer,
	markdown_renderer: MarkdownRenderer,
	#[cfg(feature = "ansi_term")]
	ansi_term_renderer: AnsiTermRenderer,
}

impl MediaRenderer {
	/// Create a renderer targeting the given media type.
	pub fn new(media_type: MediaType) -> Self {
		Self {
			media_type,
			plaintext_fallback: true,
			plain_text_renderer: PlainTextRenderer::default(),
			html_renderer: HtmlRenderer::new(),
			markdown_renderer: MarkdownRenderer::new(),
			#[cfg(feature = "ansi_term")]
			ansi_term_renderer: AnsiTermRenderer::new(),
		}
	}

	/// Disable the plain-text fallback for unrecognized text types.
	pub fn without_fallback(mut self) -> Self {
		self.plaintext_fallback = false;
		self
	}

	/// Override the [`PlainTextRenderer`] instance.
	pub fn with_plain_text_renderer(
		mut self,
		renderer: PlainTextRenderer,
	) -> Self {
		self.plain_text_renderer = renderer;
		self
	}

	/// Override the [`HtmlRenderer`] instance.
	pub fn with_html_renderer(mut self, renderer: HtmlRenderer) -> Self {
		self.html_renderer = renderer;
		self
	}

	/// Override the [`MarkdownRenderer`] instance.
	pub fn with_markdown_renderer(
		mut self,
		renderer: MarkdownRenderer,
	) -> Self {
		self.markdown_renderer = renderer;
		self
	}

	/// Override the [`AnsiTermRenderer`] instance.
	#[cfg(feature = "ansi_term")]
	pub fn with_ansi_term_renderer(
		mut self,
		renderer: AnsiTermRenderer,
	) -> Self {
		self.ansi_term_renderer = renderer;
		self
	}

	/// Change the target media type.
	pub fn with_media_type(mut self, media_type: MediaType) -> Self {
		self.media_type = media_type;
		self
	}

	/// Returns the currently configured media type.
	pub fn media_type(&self) -> &MediaType { &self.media_type }
}

impl NodeRenderer for MediaRenderer {
	fn render(&mut self, walker: &NodeWalker, entity: Entity) -> RenderOutput {
		match self.media_type {
			MediaType::Text => self.plain_text_renderer.render(walker, entity),
			MediaType::Html => self.html_renderer.render(walker, entity),
			MediaType::Markdown => {
				self.markdown_renderer.render(walker, entity)
			}
			ref other if self.plaintext_fallback && other.is_text() => {
				self.plain_text_renderer.render(walker, entity)
			}
			ref other => RenderOutput::media_string(
				other.clone(),
				format!("no renderer available for media type: {other}"),
			),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Parse HTML then render back via [`MediaRenderer`] targeting HTML.
	#[cfg(feature = "html_parser")]
	#[test]
	fn render_html() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		HtmlParser::new()
			.parse(&mut world.entity_mut(entity), b"<div>hello</div>", None)
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				MediaRenderer::new(MediaType::Html)
					.render(&walker, entity)
					.to_string()
			})
			.unwrap()
			.xpect_eq("<div>hello</div>".to_string());
	}

	/// Parse HTML then render as plain text via [`MediaRenderer`].
	#[cfg(feature = "html_parser")]
	#[test]
	fn render_plain_text() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		HtmlParser::new()
			.parse(&mut world.entity_mut(entity), b"<p>hello</p>", None)
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				MediaRenderer::new(MediaType::Text)
					.render(&walker, entity)
					.to_string()
			})
			.unwrap()
			.xpect_contains("hello");
	}

	/// Parse markdown then render back as markdown via [`MediaRenderer`].
	#[cfg(feature = "markdown_parser")]
	#[test]
	fn render_markdown() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MarkdownParser::new()
			.parse(&mut world.entity_mut(entity), b"# Title", None)
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				MediaRenderer::new(MediaType::Markdown)
					.render(&walker, entity)
					.to_string()
			})
			.unwrap()
			.trim()
			.xpect_eq("# Title");
	}

	/// A text-based media type with no dedicated renderer falls back
	/// to plain text when `plaintext_fallback` is enabled (default).
	#[cfg(feature = "html_parser")]
	#[test]
	fn fallback_text_type() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		HtmlParser::new()
			.parse(&mut world.entity_mut(entity), b"<p>hello</p>", None)
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				MediaRenderer::new(MediaType::Css)
					.render(&walker, entity)
					.to_string()
			})
			.unwrap()
			.xpect_contains("hello");
	}
}
