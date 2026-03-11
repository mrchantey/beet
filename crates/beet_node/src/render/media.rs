//! Media-type-driven renderer that dispatches to format-specific
//! [`NodeRenderer`] implementations based on the `accepts` field of
//! [`RenderContext`].
//!
//! Enable additional renderers via feature flags:
//! - `ansi_term` — adds [`AnsiTermRenderer`] support
//! - `tui` — adds [`RatatuiRenderer`] support

use crate::prelude::*;
#[allow(unused_imports)]
use beet_core::prelude::*;

/// A [`NodeRenderer`] that selects the appropriate format-specific renderer
/// based on the [`RenderContext::accepts`] list.
///
/// Iterates `accepts` in priority order, delegating to the first matching
/// renderer. If `accepts` is empty, falls back to `default_media_type`.
/// When `plaintext_fallback` is enabled, any text-based media type
/// without a dedicated renderer falls back to [`PlainTextRenderer`].
#[derive(Debug, Clone, PartialEq, Eq)]
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
	/// Buffer and area used by the TUI renderer. Callers must set
	/// these via [`Self::with_tui_buffer`] before requesting
	/// [`MediaType::Ratatui`].
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	tui_area: Option<ratatui::prelude::Rect>,
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	tui_buf: Option<ratatui::buffer::Buffer>,
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	tui_style_map: Option<StyleMap<TuiStyle>>,
}

impl Default for MediaRenderer {
	fn default() -> Self {
		#[cfg(feature = "markdown_parser")]
		let default_media_type = MediaType::Markdown;
		#[cfg(all(not(feature = "markdown_parser"), feature = "html_parser"))]
		let default_media_type = MediaType::Html;
		#[cfg(all(
			not(feature = "markdown_parser"),
			not(feature = "html_parser")
		))]
		let default_media_type = MediaType::Text;
		Self::new(default_media_type)
	}
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
			#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
			tui_area: None,
			#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
			tui_buf: None,
			#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
			tui_style_map: None,
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

	/// Set the TUI buffer and area for [`RatatuiRenderer`] output.
	///
	/// Must be called before requesting [`MediaType::Ratatui`].
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	pub fn with_tui_buffer(
		mut self,
		area: ratatui::prelude::Rect,
		buf: ratatui::buffer::Buffer,
	) -> Self {
		self.tui_area = Some(area);
		self.tui_buf = Some(buf);
		self
	}

	/// Override the [`StyleMap<TuiStyle>`] used by the TUI renderer.
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	pub fn with_tui_style_map(mut self, map: StyleMap<TuiStyle>) -> Self {
		self.tui_style_map = Some(map);
		self
	}

	/// Returns the default media type.
	pub fn default_media_type(&self) -> &MediaType { &self.default_media_type }

	/// Try to render using a specific media type, ignoring `accepts`.
	///
	/// Returns `Ok(Some(output))` on success, `Ok(None)` if no renderer
	/// handles the type, or `Err` on render failure.
	///
	/// Sub-renderers are called with an empty `accepts` list so they
	/// skip their own accept negotiation — `MediaRenderer` has already
	/// selected the correct renderer.
	fn try_render_media_type(
		&mut self,
		cx: &RenderContext,
		media_type: &MediaType,
	) -> Result<Option<RenderOutput>, RenderError> {
		// Build a context with empty accepts so sub-renderers don't
		// reject based on the original accepts list.
		let inner_cx = RenderContext::new(cx.entity, cx.walker);
		match media_type {
			MediaType::Text => {
				self.plain_text_renderer.render(&inner_cx).map(Some)
			}
			MediaType::Html => self.html_renderer.render(&inner_cx).map(Some),
			MediaType::Markdown => {
				self.markdown_renderer.render(&inner_cx).map(Some)
			}
			#[cfg(feature = "ansi_term")]
			MediaType::AnsiTerm => self.ansi_term_renderer.render(&inner_cx).map(Some),
			#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
			MediaType::Ratatui => {
				let Some(area) = self.tui_area else {
					return Err(RenderError::Other(
						bevyhow!(
							"TUI area not set; call with_tui_buffer first"
						)
						.into(),
					));
				};
				let Some(ref mut buf) = self.tui_buf else {
					return Err(RenderError::Other(
						bevyhow!(
							"TUI buffer not set; call with_tui_buffer first"
						)
						.into(),
					));
				};
				let mut renderer = RatatuiRenderer::new(area, buf);
				if let Some(map) = self.tui_style_map.take() {
					renderer = renderer.with_style_map(map);
				}
				renderer.render(&inner_cx).map(Some)
			}
			other if self.plaintext_fallback && other.is_text() => {
				self.plain_text_renderer.render(&inner_cx).map(Some)
			}
			_ => Ok(None),
		}
	}

	/// The list of media type this renderer can produce.
	fn available_types(&self) -> Vec<MediaType> {
		let mut available =
			vec![MediaType::Text, MediaType::Html, MediaType::Markdown];
		#[cfg(feature = "ansi_term")]
		available.push(MediaType::AnsiTerm);
		#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
		available.push(MediaType::Ratatui);
		available
	}
}

impl NodeRenderer for MediaRenderer {
	fn render(
		&mut self,
		cx: &RenderContext,
	) -> Result<RenderOutput, RenderError> {
		// Resolve the list of types to try: accepts list, or fallback to default.
		// Collect into an owned Vec to avoid holding a borrow on `self` during
		// the mutable dispatch calls below.
		let candidates: Vec<MediaType> = if cx.accepts.is_empty() {
			vec![self.default_media_type.clone()]
		} else {
			cx.accepts.clone()
		};

		for media_type in &candidates {
			if let Some(output) = self.try_render_media_type(cx, media_type)? {
				return Ok(output);
			}
		}

		Err(RenderError::AcceptMismatch {
			requested: candidates,
			available: self.available_types(),
		})
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
		let bytes = MediaBytes::html("<div>hello</div>");
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
		let bytes = MediaBytes::html("<p>hello</p>");
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
		let bytes = MediaBytes::markdown("# Title");
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
		let bytes = MediaBytes::html("<p>hello</p>");
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
		let bytes = MediaBytes::html("<div>hi</div>");
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

	/// When no type in `accepts` matches and fallback is disabled, errors
	/// with [`RenderError::AcceptMismatch`].
	#[cfg(feature = "html_parser")]
	#[test]
	fn no_match_errors() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::html("<div>hi</div>");
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
				match result {
					Err(RenderError::AcceptMismatch { .. }) => {}
					other => panic!("expected AcceptMismatch, got {other:?}"),
				}
			})
			.unwrap();
	}
}
