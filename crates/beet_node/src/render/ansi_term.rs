use crate::prelude::*;
use beet_core::prelude::*;
use nu_ansi_term::Color;
use nu_ansi_term::Style;
use std::borrow::Cow;

/// Renders an entity tree to styled ANSI terminal output via [`NodeVisitor`].
///
/// Maps HTML-like element names to [`Style`] values. When an element has
/// no explicit style, [`AnsiTermRenderer::default_associations`] is checked
/// for a fallback element name whose style should be reused. If neither
/// map contains an entry, [`AnsiTermRenderer::default_style`] is used.
///
/// Block-level elements emit newlines following the same rules as HTML,
/// while inline elements are rendered contiguously. Anchor tags render
/// as [OSC-8 hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiTermRenderer {
	style_map: StyleMap<Style>,
	/// Whether to clear the terminal before rendering.
	clear_on_render: bool,
	/// A string prepended to the buffer, defaults to `\n`
	prefix: Cow<'static, str>,
	/// Render [`Expression`] values verbatim as `{expr}`.
	render_expressions: bool,
	/// Whether to prefix headings with `#` markers.
	heading_hashes: bool,
	/// When `true`, decode HTML entities (eg `&amp;` → `&`) in text
	/// content via [`unescape_html_text`].
	unescape_html: bool,
	/// Shared block/inline tracking state and output buffer.
	state: TextRenderState,
}

impl Default for AnsiTermRenderer {
	fn default() -> Self { Self::new() }
}

impl AnsiTermRenderer {
	pub fn new() -> Self {
		Self {
			style_map: StyleMap::new(Style::default(), default_element_map()),
			clear_on_render: true,
			prefix: "\n".into(),
			render_expressions: false,
			heading_hashes: false,
			unescape_html: true,
			state: TextRenderState::new(),
		}
	}

	/// Override the element → style mapping.
	pub fn with_style_map(mut self, map: StyleMap<Style>) -> Self {
		self.style_map = map;
		self
	}

	/// Override the set of block-level tags.
	pub fn with_block_tags(mut self, tags: Vec<Cow<'static, str>>) -> Self {
		self.state = self.state.with_block_elements(tags);
		self
	}

	/// Enable rendering of [`Expression`] nodes as `{expr}`.
	pub fn with_expressions(mut self) -> Self {
		self.render_expressions = true;
		self
	}

	/// Override whether to clear the terminal before rendering.
	pub fn with_clear_on_render(mut self, clear: bool) -> Self {
		self.clear_on_render = clear;
		self
	}

	/// Enable HTML entity unescaping in text output.
	///
	/// When enabled, entities like `&amp;`, `&lt;`, `&gt;` are decoded
	/// to their plain-text equivalents in rendered output.
	pub fn with_unescape_html(mut self) -> Self {
		self.unescape_html = true;
		self
	}

	/// Consume the renderer and return the accumulated string.
	pub fn into_string(self) -> String { self.state.buffer }

	/// Borrow the accumulated string.
	pub fn as_str(&self) -> &str { &self.state.buffer }



	/// Write styled text to the buffer.
	fn push_styled(&mut self, text: &str) {
		let style = self.style_map.current();
		let painted = format!("{}", style.paint(text));
		self.state.push_raw(&painted);
		// push_raw tracks trailing_newline from the painted string, but the
		// ANSI codes don't contain newlines so we can track from the source
		self.state.trailing_newline = text.ends_with('\n');
	}

	/// Write an OSC-8 hyperlink opening sequence.
	fn open_osc8_link(&mut self, href: &str) {
		self.state.buffer.push_str("\x1b]8;;");
		self.state.buffer.push_str(href);
		self.state.buffer.push_str("\x1b\\");
	}

	/// Write an OSC-8 hyperlink closing sequence.
	fn close_osc8_link(&mut self) {
		self.state.buffer.push_str("\x1b]8;;\x1b\\");
	}
}


impl NodeVisitor for AnsiTermRenderer {
	fn visit_element(&mut self, _cx: &VisitContext, view: ElementView) {
		let name = view.tag();
		self.style_map.push(name);

		match name {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.state.ensure_block_separator();
				if self.heading_hashes {
					let level = name[1..].parse::<usize>().unwrap_or(1);
					let prefix = "#".repeat(level);
					// emit immediately so inline children land after the marker
					self.push_styled(&format!("{prefix} "));
				}
			}

			// ── Paragraph ──
			"p" => {
				self.state.ensure_block_separator_with_prefix(Some("▌ "));
				// emit blockquote prefix immediately so inline elements
				// (eg <em>) that open before the first text node are
				// correctly placed after the prefix
				if self.state.blockquote_depth > 0 {
					let prefix = self.state.blockquote_prefix("▌ ");
					self.push_styled(&prefix);
				}
			}

			// ── Blockquote ──
			"blockquote" => {
				self.state.ensure_block_separator_with_prefix(Some("▌ "));
				self.state.blockquote_depth += 1;
			}

			// ── Lists ──
			"ul" => {
				self.state.enter_ul();
			}
			"ol" => {
				let start = view.try_as::<OrderedListView>().unwrap().start;
				self.state.enter_ol(start);
			}
			"li" => {
				self.state.ensure_newline();
				self.state.write_list_indent();
				let prefix = self.state.next_list_prefix("• ");
				self.push_styled(&prefix);
			}

			// ── Code blocks ──
			"pre" => {
				self.state.ensure_block_separator();
				self.state.in_preformatted = true;
			}
			"code" if self.state.in_preformatted => {
				// code block content rendered directly in visit_value
			}
			"code" => {
				// inline code, style applied via style_map
			}

			// ── Links ──
			"a" => {
				let href = view.attribute_string("href");
				self.state.pending_link_href = Some(href.clone());
				self.open_osc8_link(&href);
			}

			// ── Images (void element) ──
			"img" => {
				let src = view.attribute_string("src");
				let alt = view.attribute_string("alt");
				let style = self.style_map.current();
				let display = if alt.is_empty() {
					format!("[image: {src}]")
				} else {
					format!("[{alt}]")
				};
				self.open_osc8_link(&src);
				self.state.push_raw(&format!("{}", style.paint(&display)));
				self.close_osc8_link();
			}

			// ── Thematic break ──
			"hr" => {
				self.state.ensure_block_separator();
				self.push_styled("────────────────────");
				self.state.push_raw("\n");
				self.state.needs_block_separator = true;
			}

			// ── Line break ──
			"br" => {
				self.state.push_raw("\n");
			}

			// ── Generic block handling ──
			_ => {
				if self.state.is_block_element(name) {
					self.state.ensure_block_separator();
				}
			}
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		let name = element.tag();

		match name {
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}
			"p" => {
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}
			"blockquote" => {
				self.state.blockquote_depth =
					self.state.blockquote_depth.saturating_sub(1);
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}
			"ul" | "ol" => {
				self.state.leave_list();
				if self.state.list_depth == 0 {
					self.state.ensure_newline();
					self.state.needs_block_separator = true;
				}
			}
			"li" => {
				self.state.ensure_newline();
			}
			"pre" => {
				self.state.in_preformatted = false;
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}
			"code" if !self.state.in_preformatted => {
				// inline code, style restored via style_map pop below
			}
			"a" => {
				self.close_osc8_link();
				self.state.pending_link_href = None;
			}
			// ── Images (void element, fully handled in visit_element) ──
			"img" => {}
			"hr" | "br" => {
				// already handled in visit_element
			}
			_ => {
				if self.state.is_block_element(name) {
					self.state.ensure_newline();
					self.state.needs_block_separator = true;
				}
			}
		}

		self.style_map.pop();
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		let text = value.to_string();
		if text.is_empty() {
			return;
		}

		if self.unescape_html {
			self.push_styled(&unescape_html_text(&text));
		} else {
			self.push_styled(&text);
		}
	}

	fn visit_expression(
		&mut self,
		_cx: &VisitContext,
		expression: &Expression,
	) {
		if self.render_expressions {
			let style = Style::new().italic();
			let painted =
				format!("{}", style.paint(format!("{{{}}}", expression.0)));
			self.state.push_raw(&painted);
		}
	}

	fn visit_comment(&mut self, _cx: &VisitContext, comment: &Comment) {
		self.state.ensure_block_separator();
		let style = Style::new().dimmed();
		let painted =
			format!("{}", style.paint(format!("<!--{}-->", &**comment)));
		self.state.push_raw(&painted);
		self.state.push_raw("\n");
		self.state.trailing_newline = true;
		self.state.needs_block_separator = true;
	}
}

impl NodeRenderer for AnsiTermRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<RenderOutput, RenderError> {
		cx.check_accepts(&[MediaType::AnsiTerm, MediaType::Text])?;
		cx.walk(self);
		self.state.buffer.insert_str(0, &self.prefix);
		if self.clear_on_render {
			// Clear the terminal before rendering.
			self.state.buffer.insert_str(0, "\x1b[2J\x1b[H");
		}

		RenderOutput::media_string(
			MediaType::AnsiTerm,
			std::mem::take(&mut self.state.buffer),
		)
		.xok()
	}
}


fn default_element_map() -> Vec<(&'static str, Style)> {
	vec![
		("h1", Style::new().bold().fg(Color::Green)),
		("h2", Style::new().bold().fg(Color::Cyan)),
		("h3", Style::new().bold().fg(Color::Blue)),
		("h4", Style::new().bold().fg(Color::Magenta)),
		("h5", Style::new().bold()),
		("h6", Style::new().bold().dimmed()),
		("p", Style::default()),
		("a", Style::new().fg(Color::Blue).underline()),
		("strong", Style::new().bold()),
		("em", Style::new().italic()),
		("del", Style::new().strikethrough()),
		("code", Style::new().fg(Color::Yellow)),
		("pre", Style::new().fg(Color::Yellow).dimmed()),
		("blockquote", Style::new().italic().dimmed()),
		("hr", Style::new().dimmed()),
		("img", Style::new().fg(Color::Magenta).underline()),
		("li", Style::default()),
	]
}



#[cfg(test)]
mod test {
	use super::*;

	/// Parse markdown then render via [`AnsiTermRenderer`].
	fn render(md: &str) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::markdown(md);
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		AnsiTermRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	fn strip_ansi(input: String) -> String {
		// strip_ansi takes ownership so callers can use render(...).xmap(strip_ansi)
		// strip ANSI escape sequences including OSC-8
		let mut result = String::new();
		let mut chars = input.chars().peekable();
		while let Some(ch) = chars.next() {
			if ch == '\x1b' {
				match chars.peek() {
					// OSC sequence: ESC ] ... ST (ST = ESC \)
					Some(']') => {
						chars.next();
						loop {
							match chars.next() {
								Some('\x1b') => {
									chars.next(); // consume the \ of ST
									break;
								}
								Some('\x07') => break, // BEL terminator
								None => break,
								_ => {}
							}
						}
					}
					// CSI sequence: ESC [ ... final byte
					Some('[') => {
						chars.next();
						loop {
							match chars.next() {
								Some(ch)
									if ch.is_ascii_alphabetic()
										|| ch == 'm' =>
								{
									break;
								}
								None => break,
								_ => {}
							}
						}
					}
					_ => {
						chars.next();
					}
				}
			} else {
				result.push(ch);
			}
		}
		result
	}

	#[test]
	fn render_paragraph() {
		render("Hello world")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("Hello world");
	}

	#[test]
	fn render_heading_h1() {
		// heading_hashes is false by default; only the text is emitted
		render("# Title").xmap(strip_ansi).trim().xpect_eq("Title");
	}

	#[test]
	fn render_heading_styled() {
		render("# Title")
			.xpect_contains("\x1b[")
			.xpect_contains("Title");
	}

	#[test]
	fn render_link_has_osc8() {
		render("[click](https://example.com)")
			.xpect_contains("\x1b]8;;https://example.com\x1b\\")
			.xpect_contains("click")
			.xpect_contains("\x1b]8;;\x1b\\");
	}

	#[test]
	fn render_link_text_stripped() {
		render("[click](https://example.com)")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("click");
	}

	#[test]
	fn render_code_block() {
		render("```rust\nfn main() {}\n```")
			.xmap(strip_ansi)
			.xpect_contains("fn main() {}");
	}

	#[test]
	fn render_unordered_list() {
		render("- alpha\n- beta")
			.xmap(strip_ansi)
			.trim()
			.xpect_contains("• alpha")
			.xpect_contains("• beta");
	}

	#[test]
	fn render_image() {
		render("![alt text](image.png)")
			.xmap(strip_ansi)
			.xpect_contains("[alt text]");
	}

	#[test]
	fn render_image_has_osc8() {
		render("![alt](image.png)").xpect_contains("\x1b]8;;image.png\x1b\\");
	}

	#[test]
	fn render_blockquote() {
		render("> quoted text")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("▌ quoted text");
	}

	#[test]
	fn render_blockquote_with_emphasis() {
		// inline elements inside a blockquote must appear after the prefix
		render("> *notable remark*")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("▌ notable remark");
	}

	#[test]
	fn render_blockquote_multiline() {
		// a blockquote whose content spans multiple paragraphs should prefix
		// every paragraph with ▌
		render("> first paragraph\n>\n> second paragraph")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("▌ first paragraph\n▌\n▌ second paragraph");
	}

	#[test]
	fn render_thematic_break() {
		render("---").xmap(strip_ansi).xpect_contains("────");
	}

	#[test]
	fn render_multiple_blocks_separated() {
		render("# Title\n\nParagraph")
			.xmap(strip_ansi)
			.xpect_contains("Title")
			.xpect_contains("Paragraph")
			.xpect_contains("\n\n");
	}

	#[test]
	fn unescape_html_entities() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::html("<p>a &amp; b</p>");
		HtmlParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		AnsiTermRenderer::new()
			.with_unescape_html()
			.with_clear_on_render(false)
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
			.xmap(strip_ansi)
			.xpect_contains("a & b");
	}

	#[test]
	fn custom_style_map() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::markdown("# Hello");
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		AnsiTermRenderer::new()
			.with_style_map(StyleMap::new(Style::default(), vec![(
				"h1",
				Style::new().fg(Color::Red),
			)]))
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
			.xpect_contains("\x1b[")
			.xpect_contains("Hello");
	}
}
