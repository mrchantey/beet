use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;

/// Renders an entity tree to styled ANSI terminal output via [`NodeVisitor`].
///
/// Output is a sequential string of arbitrary length, so a [character-cell
/// buffer](crate::render::charcell) is not used: block elements emit newlines
/// following HTML rules while inline elements render contiguously, and anchor
/// tags render as
/// [OSC-8 hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda).
///
/// All styling comes from the resolved [`VisualStyle`] components populated by
/// the [`StylePlugin`] during the [`PostParseTree`] schedule: prose defaults
/// (`em` → italic, `h1` → bold colour) come from
/// [`default_element_rules`](crate::style::default_element_rules) and
/// tree-sitter syntax highlighting from `apply_syntax_highlighting`'s
/// `class="hl-<capture>"` spans. The renderer holds no style table of its own —
/// it simply reads the style resolved for each entity. Without a
/// [`StylePlugin`] the structure is still rendered, just unstyled.
#[derive(Debug, Clone, PartialEq)]
pub struct AnsiTermRenderer {
	/// Whether to clear the terminal before rendering.
	clear_on_render: bool,
	/// A string prepended to the buffer, defaults to `\n`.
	prefix: Cow<'static, str>,
	/// Render [`Expression`] values verbatim as `{expr}`.
	render_expressions: bool,
	/// Whether to prefix headings with `#` markers.
	heading_hashes: bool,
	/// Shared block/inline tracking state and output buffer.
	state: TextRenderState,
	/// Resolved [`VisualStyle`] per entity, snapshotted before walking.
	resolved: HashMap<Entity, VisualStyle>,
}

impl Default for AnsiTermRenderer {
	fn default() -> Self { Self::new() }
}

impl AnsiTermRenderer {
	pub fn new() -> Self {
		Self {
			clear_on_render: true,
			prefix: "\n".into(),
			render_expressions: false,
			heading_hashes: false,
			state: TextRenderState::new(),
			resolved: HashMap::default(),
		}
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

	/// Prefix headings with `#` markers matching their level.
	pub fn with_heading_hashes(mut self) -> Self {
		self.heading_hashes = true;
		self
	}

	/// Override whether to clear the terminal before rendering.
	pub fn with_clear_on_render(mut self, clear: bool) -> Self {
		self.clear_on_render = clear;
		self
	}

	/// Consume the renderer and return the accumulated string.
	pub fn into_string(self) -> String { self.state.buffer }

	/// Borrow the accumulated string.
	pub fn as_str(&self) -> &str { &self.state.buffer }

	/// The resolved [`VisualStyle`] for `entity`, or the default when no
	/// styles were resolved (eg no [`StylePlugin`]).
	fn style_for(&self, entity: Entity) -> VisualStyle {
		self.resolved.get(&entity).cloned().unwrap_or_default()
	}

	/// Write `text` with the resolved style for `entity`.
	fn push_styled(&mut self, entity: Entity, text: &str) {
		self.write_with_style(&self.style_for(entity), text);
	}

	fn write_with_style(&mut self, style: &VisualStyle, text: &str) {
		let mut buf: Vec<u8> = Vec::new();
		style.write_style(&mut buf, None).ok();
		buf.extend_from_slice(text.as_bytes());
		buf.extend_from_slice(escape::RESET.as_bytes());
		self.state.push_raw(&String::from_utf8_lossy(&buf));
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

	/// Snapshot the resolved [`VisualStyle`] of every entity in the world so
	/// the borrow-free [`NodeVisitor`] walk can read them.
	fn snapshot_styles(&mut self, world: &mut World) {
		self.resolved = world
			.query::<(Entity, &VisualStyle)>()
			.iter(world)
			.map(|(entity, style)| (entity, style.clone()))
			.collect();
	}
}

impl NodeVisitor for AnsiTermRenderer {
	fn visit_element(&mut self, cx: &VisitContext, view: ElementView) {
		let name = view.tag();

		match name {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.state.ensure_block_separator();
				if self.heading_hashes {
					let level = name[1..].parse::<usize>().unwrap_or(1);
					let prefix = "#".repeat(level);
					self.push_styled(cx.entity, &format!("{prefix} "));
				}
			}

			// ── Paragraph ──
			"p" => {
				self.state.ensure_block_separator_with_prefix(Some("▌ "));
				if self.state.blockquote_depth > 0 {
					let prefix = self.state.blockquote_prefix("▌ ");
					self.push_styled(cx.entity, &prefix);
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
				self.push_styled(cx.entity, &prefix);
			}

			// ── Code blocks ──
			"pre" => {
				self.state.ensure_block_separator();
				self.state.in_preformatted = true;
			}
			"code" => {
				// styling resolved per text child via the RuleSet
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
				let display = if alt.is_empty() {
					format!("[image: {src}]")
				} else {
					format!("[{alt}]")
				};
				self.open_osc8_link(&src);
				self.push_styled(cx.entity, &display);
				self.close_osc8_link();
			}

			// ── Thematic break ──
			"hr" => {
				self.state.ensure_block_separator();
				self.push_styled(cx.entity, "────────────────────");
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
			"a" => {
				self.close_osc8_link();
				self.state.pending_link_href = None;
			}
			"hr" | "br" | "img" => {
				// fully handled in visit_element
			}
			_ => {
				if self.state.is_block_element(name) {
					self.state.ensure_newline();
					self.state.needs_block_separator = true;
				}
			}
		}
	}

	fn visit_value(&mut self, cx: &VisitContext, value: &Value) {
		let text = value.to_string();
		if text.is_empty() {
			return;
		}
		self.push_styled(cx.entity, &text);
	}

	fn visit_expression(
		&mut self,
		_cx: &VisitContext,
		expression: &Expression,
	) {
		if self.render_expressions {
			let style = VisualStyle {
				text_style: TextStyle::ITALIC,
				..VisualStyle::default()
			};
			self.write_with_style(&style, &format!("{{{}}}", expression.0));
		}
	}

	fn visit_comment(&mut self, _cx: &VisitContext, comment: &Comment) {
		self.state.ensure_block_separator();
		let style = VisualStyle {
			foreground: Some(Color::srgba(1., 1., 1., 0.4)),
			..VisualStyle::default()
		};
		self.write_with_style(&style, &format!("<!--{}-->", &**comment));
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
		self.snapshot_styles(cx.world);
		cx.walk(self);
		self.state.buffer.insert_str(0, &self.prefix);
		if self.clear_on_render {
			self.state.buffer.insert_str(0, "\x1b[2J\x1b[H");
		}

		RenderOutput::media_string(
			MediaType::AnsiTerm,
			std::mem::take(&mut self.state.buffer),
		)
		.xok()
	}
}


#[cfg(test)]
#[cfg(feature = "markdown_parser")]
mod test {
	use super::*;

	/// Parse markdown, resolve styles, then render via [`AnsiTermRenderer`].
	fn render(md: &str) -> String {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let entity = app.world_mut().spawn_empty().id();
		let bytes = MediaBytes::new_markdown(md);
		MarkdownParser::new()
			.parse(ParseContext::new(
				&mut app.world_mut().entity_mut(entity),
				&bytes,
			))
			.unwrap();
		AnsiTermRenderer::new()
			.with_clear_on_render(false)
			.render(&mut RenderContext::new(entity, app.world_mut()))
			.unwrap()
			.to_string()
	}

	/// Strip ANSI escape sequences (CSI and OSC-8), leaving visible text.
	fn strip_ansi(input: String) -> String {
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
								Some('\x07') | None => break,
								_ => {}
							}
						}
					}
					// CSI sequence: ESC [ ... final byte
					Some('[') => {
						chars.next();
						loop {
							match chars.next() {
								Some(ch) if ch.is_ascii_alphabetic() => break,
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

	#[beet_core::test]
	fn render_paragraph() {
		render("Hello world")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("Hello world");
	}

	#[beet_core::test]
	fn render_heading_text() {
		// heading_hashes is false by default; only the text is emitted
		render("# Title").xmap(strip_ansi).trim().xpect_eq("Title");
	}

	#[beet_core::test]
	fn render_heading_styled() {
		render("# Title")
			.xpect_contains("\x1b[")
			.xpect_contains("Title");
	}

	#[beet_core::test]
	fn render_link_has_osc8() {
		render("[click](https://example.com)")
			.xpect_contains("\x1b]8;;https://example.com\x1b\\")
			.xpect_contains("click")
			.xpect_contains("\x1b]8;;\x1b\\");
	}

	#[beet_core::test]
	fn render_link_text_stripped() {
		render("[click](https://example.com)")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("click");
	}

	#[beet_core::test]
	fn render_code_block() {
		render("```rust\nfn main() {}\n```")
			.xmap(strip_ansi)
			.xpect_contains("fn main() {}");
	}

	#[cfg(feature = "syntax_highlighting")]
	#[beet_core::test]
	fn render_code_block_syntax_styled() {
		// the `fn` keyword resolves to a syntax-highlight foreground colour
		// through the default theme, with no per-renderer configuration
		render("```rust\nfn main() {}\n```").xpect_contains("\x1b[");
	}

	#[beet_core::test]
	fn render_unordered_list() {
		render("- alpha\n- beta")
			.xmap(strip_ansi)
			.trim()
			.xpect_contains("• alpha")
			.xpect_contains("• beta");
	}

	#[beet_core::test]
	fn render_image() {
		render("![alt text](image.png)")
			.xmap(strip_ansi)
			.xpect_contains("[alt text]");
	}

	#[beet_core::test]
	fn render_image_has_osc8() {
		render("![alt](image.png)").xpect_contains("\x1b]8;;image.png\x1b\\");
	}

	#[beet_core::test]
	fn render_blockquote() {
		render("> quoted text")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("▌ quoted text");
	}

	#[beet_core::test]
	fn render_blockquote_with_emphasis() {
		// inline elements inside a blockquote must appear after the prefix
		render("> *notable remark*")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("▌ notable remark");
	}

	#[beet_core::test]
	fn render_thematic_break() {
		render("---").xmap(strip_ansi).xpect_contains("────");
	}

	#[beet_core::test]
	fn render_multiple_blocks_separated() {
		render("# Title\n\nParagraph")
			.xmap(strip_ansi)
			.xpect_contains("Title")
			.xpect_contains("Paragraph")
			.xpect_contains("\n\n");
	}

	#[cfg(feature = "html_parser")]
	#[beet_core::test]
	fn unescape_html_entities() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let entity = app.world_mut().spawn_empty().id();
		let bytes = MediaBytes::new_html("<p>a &amp; b</p>");
		HtmlParser::new()
			.parse(ParseContext::new(
				&mut app.world_mut().entity_mut(entity),
				&bytes,
			))
			.unwrap();
		AnsiTermRenderer::new()
			.with_clear_on_render(false)
			.render(&mut RenderContext::new(entity, app.world_mut()))
			.unwrap()
			.to_string()
			.xmap(strip_ansi)
			.xpect_contains("a & b");
	}
}
