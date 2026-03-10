//! Media-type-driven renderer that dispatches to format-specific
//! [`NodeRenderer`] implementations based on the `accepts` field of
//! [`RenderContext`].
//!
//! Enable additional renderers via feature flags:
//! - `ansi_term` — adds [`AnsiTermRenderer`] support

use crate::prelude::*;
use beet_core::prelude::*;

/// A [`NodeRenderer`] that selects the appropriate format-specific renderer
/// based on the [`RenderContext::accepts`] list.
///
/// Iterates `accepts` in priority order, delegating to the first matching
/// renderer. If `accepts` is empty, falls back to `default_media_type`.
/// When `plaintext_fallback` is enabled, any text-based media type
/// without a dedicated renderer falls back to [`PlainTextRenderer`].
pub struct MediaRenderer {
	/// Used when [`RenderContext::accepts`] is empty.
	default_media_type: MediaType,
	/// Fall back to [`PlainTextRenderer`] for unrecognized text types.
	plaintext_fallback: bool,
	plain_text_renderer: PlainTextRenderer,
	html_renderer: HtmlRenderer,
	markdown_renderer: MarkdownRenderer,
	#[cfg(feature = "ansi_term")]
	ansi_term_renderer: AnsiTermRenderer,
}

impl MediaRenderer {
	/// Create a renderer with the given default media type.
	pub fn new(default_media_type: MediaType) -> Self {
		Self {
			default_media_type,
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

	/// Set the default media type used when `accepts` is empty.
	pub fn with_default_media_type(mut self, media_type: MediaType) -> Self {
		self.default_media_type = media_type;
		self
	}

	/// Returns the default media type.
	pub fn default_media_type(&self) -> &MediaType { &self.default_media_type }

	/// Render using a specific media type, ignoring `accepts`.
	fn render_media_type(
		&mut self,
		cx: &RenderContext,
		media_type: &MediaType,
	) -> Option<Result<RenderOutput>> {
		match media_type {
			MediaType::Text => Some(self.plain_text_renderer.render(cx)),
			MediaType::Html => Some(self.html_renderer.render(cx)),
			MediaType::Markdown => Some(self.markdown_renderer.render(cx)),
			#[cfg(feature = "ansi_term")]
			// ansi_term renders as text/plain with ANSI escape codes
			_ if *media_type == MediaType::Text => {
				Some(self.ansi_term_renderer.render(cx))
			}
			other if self.plaintext_fallback && other.is_text() => {
				Some(self.plain_text_renderer.render(cx))
			}
			_ => None,
		}
	}
}

impl NodeRenderer for MediaRenderer {
	fn render(&mut self, cx: &RenderContext) -> Result<RenderOutput> {
		// Resolve the list of types to try: accepts list, or fallback to default.
		// Collect into an owned Vec to avoid holding a borrow on `self` during
		// the mutable dispatch calls below.
		let candidates: Vec<MediaType> = if cx.accepts.is_empty() {
			vec![self.default_media_type.clone()]
		} else {
			cx.accepts.clone()
		};

		for media_type in &candidates {
			if let Some(result) = self.render_media_type(cx, media_type) {
				return result;
			}
		}

		// Nothing matched — report what was requested vs what is available.
		let available =
			vec![MediaType::Text, MediaType::Html, MediaType::Markdown];
		bevybail!(
			"no renderer available for any of the requested types: {:?}, available: {:?}",
			candidates,
			available
		)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn render(
		world: &mut World,
		entity: Entity,
		default_media_type: MediaType,
		accepts: Vec<MediaType>,
	) -> String {
		world
			.run_system_once(move |walker: NodeWalker| {
				let cx = RenderContext::new(entity, &walker)
					.with_accepts(accepts.clone());
				MediaRenderer::new(default_media_type.clone())
					.render(&cx)
					.unwrap()
					.to_string()
			})
			.unwrap()
	}

	/// Parse HTML then render back via [`MediaRenderer`] targeting HTML.
	#[cfg(feature = "html_parser")]
	#[test]
	fn render_html() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::from_str(MediaType::Html, "<div>hello</div>");
		HtmlParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		render(&mut world, entity, MediaType::Html, vec![])
			.xpect_eq("<div>hello</div>".to_string());
	}

	/// Parse HTML then render as plain text via [`MediaRenderer`].
	#[cfg(feature = "html_parser")]
	#[test]
	fn render_plain_text() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::from_str(MediaType::Html, "<p>hello</p>");
		HtmlParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		render(&mut world, entity, MediaType::Text, vec![])
			.xpect_contains("hello");
	}

	/// Parse markdown then render back as markdown via [`MediaRenderer`].
	#[cfg(feature = "markdown_parser")]
	#[test]
	fn render_markdown() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::from_str(MediaType::Markdown, "# Title");
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		render(&mut world, entity, MediaType::Markdown, vec![])
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
		let bytes = MediaBytes::from_str(MediaType::Html, "<p>hello</p>");
		HtmlParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		render(&mut world, entity, MediaType::Css, vec![])
			.xpect_contains("hello");
	}

	/// `accepts` list is consulted in priority order.
	#[cfg(feature = "html_parser")]
	#[test]
	fn accepts_priority() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::from_str(MediaType::Html, "<div>hi</div>");
		HtmlParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		// Html is first in accepts, so we should get HTML output.
		render(&mut world, entity, MediaType::Text, vec![
			MediaType::Html,
			MediaType::Text,
		])
		.xpect_eq("<div>hi</div>".to_string());
	}

	/// When no type in `accepts` matches and fallback is disabled, errors.
	#[cfg(feature = "html_parser")]
	#[test]
	fn no_match_errors() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::from_str(MediaType::Html, "<div>hi</div>");
		HtmlParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		// Render with only Png accepted and fallback disabled — should error.
		world
			.run_system_once(move |walker: NodeWalker| {
				let cx = RenderContext::new(entity, &walker)
					.with_accepts(vec![MediaType::Png]);
				let result = MediaRenderer::new(MediaType::Text)
					.without_fallback()
					.render(&cx);
				result.is_err().xpect_true();
			})
			.unwrap();
	}
}
