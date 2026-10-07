use crate::prelude::*;
use alloc::borrow::Cow;
use beet_core::prelude::*;

/// Renders an entity tree back to a markdown string via [`NodeVisitor`].
///
/// Converts HTML-like element trees (as produced by [`MarkdownParser`])
/// back into CommonMark-compatible markdown. Supports headings, emphasis,
/// strong, links, images, lists, code blocks, blockquotes, thematic
/// breaks, inline code, GFM tables, and optional expression rendering. A
/// `<mark>`, and a `<span>` carrying a `style`, pass through as inline HTML,
/// since markdown has no syntax for what they signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownRenderer {
	/// Shared block/inline tracking state and output buffer.
	state: TextRenderState,
	/// Render [`Expression`] values verbatim as `{expr}` in output.
	render_expressions: bool,
	/// Stack of active inline wrappers to emit on leave.
	inline_stack: Vec<InlineWrapper>,
	/// The tables being collected, innermost last.
	tables: Vec<TableCapture>,
}

/// A table being collected, written as GFM when it closes, since a row is
/// only known once every cell in it is.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct TableCapture {
	/// Where the table starts in the buffer.
	start: usize,
	/// The rows so far, each cell's text.
	rows: Vec<Vec<String>>,
	/// Where the open cell's text starts in the buffer, and its column span.
	cell: Option<(usize, usize)>,
}

impl TableCapture {
	/// A cell's rendered text as one GFM cell: its lines joined by `<br>`,
	/// its pipes escaped.
	fn cell_text(text: &str) -> String {
		text.lines()
			.map(str::trim)
			.filter(|line| !line.is_empty())
			.collect::<Vec<_>>()
			.join("<br>")
			.replace('|', "\\|")
	}

	/// The table as GFM, its first row the header, every row padded to the
	/// widest.
	fn to_markdown(&self) -> String {
		let width = self.rows.iter().map(Vec::len).max().unwrap_or(0);
		if width == 0 {
			return String::new();
		}
		let line = |row: &Vec<String>| {
			let cells = (0..width)
				.map(|index| {
					row.get(index).map(String::as_str).unwrap_or_default()
				})
				.collect::<Vec<_>>();
			format!("| {} |\n", cells.join(" | "))
		};
		let mut out = line(&self.rows[0]);
		out.push_str(&format!("|{}\n", "---|".repeat(width)));
		for row in &self.rows[1..] {
			out.push_str(&line(row));
		}
		out
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineWrapper {
	Em,
	Strong,
	Del,
	InlineCode,
	/// Link wrapper deferred until leave.
	Link,
	/// Superscript.
	Sup,
	/// Subscript.
	Sub,
	/// Inline HTML, closed by this tag.
	Html(&'static str),
}

impl Default for MarkdownRenderer {
	fn default() -> Self { Self::new() }
}

impl MarkdownRenderer {
	pub fn new() -> Self {
		Self {
			state: TextRenderState::new(),
			render_expressions: false,
			inline_stack: Vec::new(),
			tables: Vec::new(),
		}
	}

	/// Enable rendering of [`Expression`] nodes as `{expr}`.
	pub fn with_expressions(mut self) -> Self {
		self.render_expressions = true;
		self
	}

	/// Override the set of block-level elements.
	pub fn with_block_elements(
		mut self,
		elements: Vec<Cow<'static, str>>,
	) -> Self {
		self.state = self.state.with_block_elements(elements);
		self
	}

	/// Consume the renderer and return the accumulated markdown string.
	pub fn into_string(self) -> String { self.state.buffer }

	/// Borrow the accumulated markdown string.
	pub fn as_str(&self) -> &str { &self.state.buffer }

	fn push_str(&mut self, text: &str) { self.state.push_raw(text); }

	fn push_char(&mut self, ch: char) { self.state.push_raw_char(ch); }

	/// Opens an inline HTML element, its `style` kept: `mark` always, any
	/// other only when styled, since an unstyled `span` signals nothing.
	fn open_html(&mut self, view: &ElementView, close: &'static str) {
		let style = view.attribute_string("style");
		match (view.tag(), style.is_empty()) {
			("mark", true) => self.push_str("<mark>"),
			(tag, false) => self.push_str(&format!(
				"<{tag} style=\"{}\">",
				style.replace('"', "&quot;")
			)),
			_ => {
				self.inline_stack.push(InlineWrapper::Html(""));
				return;
			}
		}
		self.inline_stack.push(InlineWrapper::Html(close));
	}
}

impl NodeVisitor for MarkdownRenderer {
	fn visit_element(&mut self, cx: &VisitContext, view: ElementView) {
		let name = view.tag();
		let value = view.value;

		match name {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.state.ensure_block_separator();
				let level = name.as_bytes()[1] - b'0';
				let prefix = "#".repeat(level as usize);
				self.push_str(&format!("{prefix} "));
			}

			// ── Paragraph ──
			"p" => {
				self.state.ensure_block_separator_with_prefix(Some("> "));
				// emit blockquote prefix immediately so inline elements
				// (eg <em>) that open before the first text node are
				// correctly placed after the prefix
				if self.state.blockquote_depth > 0 {
					let prefix = self.state.blockquote_prefix("> ");
					self.push_str(&prefix);
				}
			}

			// ── Blockquote ──
			"blockquote" => {
				self.state.ensure_block_separator_with_prefix(Some("> "));
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
				let prefix = self.state.next_list_prefix("- ");
				self.push_str(&prefix);
			}

			// ── Code blocks ──
			"pre" => {
				self.state.ensure_block_separator();
				self.state.in_preformatted = true;
			}
			"code" if self.state.in_preformatted => {
				// fenced code block: extract language from class
				let info = view
					.attribute("class")
					.and_then(|attr| match attr.value {
						Value::Str(class) => class
							.strip_prefix("language-")
							.map(|lang| lang.to_string())
							.or_else(|| Some(class.to_string())),
						_ => None,
					})
					.unwrap_or_default();
				self.state.code_fence_info = Some(info.clone());
				self.push_str("```");
				self.push_str(&info);
				self.push_char('\n');
			}
			"code" => {
				// inline code
				self.push_char('`');
				self.inline_stack.push(InlineWrapper::InlineCode);
			}

			// ── Inline formatting ──
			"em" | "i" => {
				self.push_char('*');
				self.inline_stack.push(InlineWrapper::Em);
			}
			"strong" | "b" => {
				self.push_str("**");
				self.inline_stack.push(InlineWrapper::Strong);
			}
			"del" | "s" => {
				self.push_str("~~");
				self.inline_stack.push(InlineWrapper::Del);
			}
			"sup" => {
				self.push_char('^');
				self.inline_stack.push(InlineWrapper::Sup);
			}
			"sub" => {
				self.push_char('~');
				self.inline_stack.push(InlineWrapper::Sub);
			}

			// ── Links ──
			"a" => {
				let href = view.attribute_string("href");
				self.state.pending_link_href = Some(href);
				self.push_char('[');
				self.inline_stack.push(InlineWrapper::Link);
			}

			// ── Images (void element) ──
			"img" => {
				let src = view.attribute_string("src");
				let alt = view.attribute_string("alt");
				self.push_str(&format!("![{}]({})", alt, src));
			}

			// ── Thematic break ──
			"hr" => {
				self.state.ensure_block_separator();
				self.push_str("---");
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}

			// ── Line break ──
			"br" => {
				self.push_str("  \n");
			}

			// ── Inline HTML ──
			"mark" => self.open_html(&view, "</mark>"),
			"span" => self.open_html(&view, "</span>"),

			// ── Tables, collected and written on leave ──
			"table" => {
				self.state.ensure_block_separator();
				self.tables.push(TableCapture {
					start: self.state.buffer.len(),
					..default()
				});
			}
			"tr" => {
				if let Some(table) = self.tables.last_mut() {
					table.rows.push(Vec::new());
				}
			}
			"th" | "td" => {
				let span = view
					.attribute_string("colspan")
					.parse::<usize>()
					.unwrap_or(1)
					.max(1);
				let start = self.state.buffer.len();
				if let Some(table) = self.tables.last_mut() {
					table.cell = Some((start, span));
				}
			}
			"thead" | "tbody" | "tfoot" => {}

			// ── Catch-all for unknown block/inline elements ──
			_ => {
				if self.state.is_block_element(name) {
					self.state.ensure_block_separator();
				}
			}
		}
		// a control's typed value is its text
		if let Some(value) = value {
			self.visit_value(cx, value);
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		let name = element.tag();

		match name {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}

			// ── Paragraph ──
			"p" => {
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}

			// ── Blockquote ──
			"blockquote" => {
				self.state.blockquote_depth =
					self.state.blockquote_depth.saturating_sub(1);
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}

			// ── Lists ──
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

			// ── Code blocks ──
			"code" if self.state.in_preformatted => {
				self.state.ensure_newline();
				self.push_str("```");
				self.push_char('\n');
				self.state.code_fence_info = None;
			}
			"code" => {
				if let Some(InlineWrapper::InlineCode) =
					self.inline_stack.last()
				{
					self.inline_stack.pop();
				}
				self.push_char('`');
			}
			"pre" => {
				self.state.in_preformatted = false;
				self.state.needs_block_separator = true;
			}

			// ── Inline formatting ──
			"em" | "i" => {
				if let Some(InlineWrapper::Em) = self.inline_stack.last() {
					self.inline_stack.pop();
				}
				self.push_char('*');
			}
			"strong" | "b" => {
				if let Some(InlineWrapper::Strong) = self.inline_stack.last() {
					self.inline_stack.pop();
				}
				self.push_str("**");
			}
			"del" | "s" => {
				if let Some(InlineWrapper::Del) = self.inline_stack.last() {
					self.inline_stack.pop();
				}
				self.push_str("~~");
			}
			"sup" => {
				if let Some(InlineWrapper::Sup) = self.inline_stack.last() {
					self.inline_stack.pop();
				}
				self.push_char('^');
			}
			"sub" => {
				if let Some(InlineWrapper::Sub) = self.inline_stack.last() {
					self.inline_stack.pop();
				}
				self.push_char('~');
			}

			// ── Links ──
			"a" => {
				if let Some(InlineWrapper::Link) = self.inline_stack.last() {
					self.inline_stack.pop();
				}
				let href =
					self.state.pending_link_href.take().unwrap_or_default();
				self.push_str("](");
				self.push_str(&href);
				self.push_char(')');
			}

			// ── Inline HTML ──
			"mark" | "span" => {
				if let Some(InlineWrapper::Html(close)) =
					self.inline_stack.last()
				{
					let close = *close;
					self.inline_stack.pop();
					self.push_str(close);
				}
			}

			// ── Tables ──
			"th" | "td" => {
				let Some((start, span)) =
					self.tables.last_mut().and_then(|table| table.cell.take())
				else {
					return;
				};
				let text =
					TableCapture::cell_text(&self.state.take_from(start));
				self.state.needs_block_separator = false;
				if let Some(table) = self.tables.last_mut() {
					if table.rows.is_empty() {
						table.rows.push(Vec::new());
					}
					let row = table.rows.last_mut().unwrap();
					row.push(text);
					row.extend((1..span).map(|_| String::new()));
				}
			}
			"table" => {
				let Some(table) = self.tables.pop() else {
					return;
				};
				// whatever the table held outside its cells is dropped
				self.state.take_from(table.start);
				self.push_str(&table.to_markdown());
				self.state.ensure_newline();
				self.state.needs_block_separator = true;
			}
			"tr" | "thead" | "tbody" | "tfoot" => {}

			// ── Images (void element, fully handled in visit_element) ──
			"img" => {}

			// ── Void/self-closing ──
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
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		let text = value.to_string();
		if text.is_empty() {
			return;
		}
		self.push_str(&text);
	}

	fn visit_expression(
		&mut self,
		_cx: &VisitContext,
		expression: &Expression,
	) {
		if self.render_expressions {
			self.push_char('{');
			self.push_str(&expression.0);
			self.push_char('}');
		}
	}

	fn visit_comment(&mut self, _cx: &VisitContext, comment: &Comment) {
		self.state.ensure_block_separator();
		self.push_str("<!--");
		self.push_str(comment);
		self.push_str("-->");
		self.state.ensure_newline();
		self.state.needs_block_separator = true;
	}
}

impl NodeRenderer for MarkdownRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		cx.check_accepts(&[MediaType::Markdown])?;
		cx.walk(self);
		MediaBytes::new_string(
			MediaType::Markdown,
			core::mem::take(&mut self.state.buffer),
		)
		.xok()
	}
}

#[cfg(test)]
#[cfg(feature = "markdown_parser")]
mod test {
	use super::*;

	/// Parse markdown then render it back via [`MarkdownRenderer`].
	fn roundtrip(md: &str) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new_markdown(md);
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		MarkdownRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	/// Parse markdown then render with expression support.
	#[allow(dead_code)]
	fn roundtrip_expressions(md: &str) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new_markdown(md);
		MarkdownParser::with_expressions()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		MarkdownRenderer::new()
			.with_expressions()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn render_paragraph() {
		roundtrip("Hello world").trim().xpect_eq("Hello world");
	}

	#[beet_core::test]
	fn render_heading_h1() { roundtrip("# Title").trim().xpect_eq("# Title"); }

	#[beet_core::test]
	fn render_heading_h2() {
		roundtrip("## Subtitle").trim().xpect_eq("## Subtitle");
	}

	#[beet_core::test]
	fn render_emphasis() { roundtrip("*hello*").trim().xpect_eq("*hello*"); }

	#[beet_core::test]
	fn render_strong() { roundtrip("**hello**").trim().xpect_eq("**hello**"); }

	#[beet_core::test]
	fn render_link() {
		roundtrip("[click](https://example.com)")
			.trim()
			.xpect_eq("[click](https://example.com)");
	}

	#[beet_core::test]
	fn render_image() {
		roundtrip("![alt](image.png)")
			.trim()
			.xpect_eq("![alt](image.png)");
	}

	#[beet_core::test]
	fn render_unordered_list() {
		roundtrip("- alpha\n- beta")
			.trim()
			.xpect_eq("- alpha\n- beta");
	}

	#[beet_core::test]
	fn render_code_block() {
		roundtrip("```rust\nfn main() {}\n```")
			.trim()
			.xpect_eq("```rust\nfn main() {}\n```");
	}

	#[beet_core::test]
	fn render_inline_code() {
		roundtrip("use `foo()` here")
			.trim()
			.xpect_eq("use `foo()` here");
	}

	#[beet_core::test]
	fn render_blockquote() {
		roundtrip("> quoted text").trim().xpect_eq("> quoted text");
	}

	#[beet_core::test]
	fn render_blockquote_with_emphasis() {
		// inline elements inside a blockquote must appear after the prefix
		roundtrip("> *notable remark*")
			.trim()
			.xpect_eq("> *notable remark*");
	}

	#[beet_core::test]
	fn render_blockquote_multiline() {
		let input = "> first paragraph\n>\n> second paragraph";
		roundtrip(input).trim().xpect_eq(input);
	}

	#[beet_core::test]
	fn render_thematic_break() { roundtrip("---").trim().xpect_eq("---"); }

	#[beet_core::test]
	fn render_multiple_blocks() {
		roundtrip("# Title\n\nParagraph")
			.trim()
			.xpect_eq("# Title\n\nParagraph");
	}

	#[beet_core::test]
	fn render_comment() {
		roundtrip("<!-- hello -->")
			.trim()
			.xpect_eq("<!-- hello -->");
	}

	#[cfg(feature = "bsx")]
	/// Parse HTML (with entities), then render as markdown.
	fn render_unescaped(html: &str) -> String {
		let mut world = world_ext::ui_world();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new_html(html);
		BsxParser::html()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		MarkdownRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn unescape_html_entities_in_text() {
		render_unescaped("<p>a &amp; b</p>")
			.trim()
			.xpect_eq("a & b");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn unescape_angle_brackets_in_text() {
		render_unescaped("<p>&lt;div&gt;</p>")
			.trim()
			.xpect_eq("<div>");
	}

	#[beet_core::test]
	fn render_table() {
		roundtrip("| a | b |\n|---|---|\n| c | d \\| e |")
			.trim()
			.xpect_eq("| a | b |\n|---|---|\n| c | d \\| e |");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_html_table() {
		render_unescaped(
			"<p>before</p><table><tr><td><p>one</p><p>two</p></td><td colspan=\"2\">x</td></tr><tr><td>a</td></tr></table><p>after</p>",
		)
		.trim()
		.xpect_eq("before\n\n| one<br>two | x |  |\n|---|---|---|\n| a |  |  |\n\nafter");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_inline_html() {
		render_unescaped(
			"<p><mark>due</mark> <span>plain</span> <span style=\"color: #FF0000\">red</span></p>",
		)
		.trim()
		.xpect_eq("<mark>due</mark> plain <span style=\"color: #FF0000\">red</span>");
	}

	#[beet_core::test]
	fn plain_text_passes_through() {
		roundtrip("hello world").trim().xpect_eq("hello world");
	}
}
