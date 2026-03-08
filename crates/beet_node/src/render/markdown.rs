use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;

/// Renders an entity tree back to a markdown string via [`NodeVisitor`].
///
/// Converts HTML-like element trees (as produced by [`MarkdownParser`])
/// back into CommonMark-compatible markdown. Supports headings, emphasis,
/// strong, links, images, lists, code blocks, blockquotes, thematic
/// breaks, inline code, and optional expression rendering.
pub struct MarkdownRenderer {
	/// Shared block/inline tracking state and output buffer.
	state: TextRenderState,
	/// Render [`Expression`] values verbatim as `{expr}` in output.
	render_expressions: bool,
	/// Stack of active inline wrappers to emit on leave.
	inline_stack: Vec<InlineWrapper>,
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
}


impl NodeVisitor for MarkdownRenderer {
	fn visit_element(
		&mut self,
		_cx: &VisitContext,
		element: &Element,
		attrs: Vec<(Entity, &Attribute, &Value)>,
	) {
		let name = element.name().to_ascii_lowercase();

		match name.as_str() {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.state.ensure_block_separator();
				let level = name[1..].parse::<usize>().unwrap_or(1);
				let prefix = "#".repeat(level);
				self.push_str(&format!("{prefix} "));
			}

			// ── Paragraph ──
			"p" => {
				self.state.ensure_block_separator();
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
				self.state.ensure_block_separator();
				self.state.blockquote_depth += 1;
			}

			// ── Lists ──
			"ul" => {
				self.state.enter_ul();
			}
			"ol" => {
				let start = TextRenderState::ol_start(&attrs);
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
				let info = TextRenderState::find_attr(&attrs, "class")
					.and_then(|val| match val {
						Value::Str(class) => class
							.strip_prefix("language-")
							.map(|lang| lang.to_string())
							.or_else(|| Some(class.clone())),
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
				let href = TextRenderState::find_attr(&attrs, "href")
					.map(|val| val.to_string())
					.unwrap_or_default();
				self.state.pending_link_href = Some(href);
				self.push_char('[');
				self.inline_stack.push(InlineWrapper::Link);
			}

			// ── Images ──
			"img" => {
				let src = TextRenderState::find_attr(&attrs, "src")
					.map(|val| val.to_string())
					.unwrap_or_default();
				self.state.image_src = Some(src);
				self.state.image_alt = Some(String::new());
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

			// ── Tables ──
			"table" | "thead" | "tbody" | "tr" | "th" | "td" => {
				// text content flows through visit_value
			}

			// ── Catch-all for unknown block/inline elements ──
			_ => {
				if self.state.is_block_element(&name) {
					self.state.ensure_block_separator();
				}
			}
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		let name = element.name().to_ascii_lowercase();

		match name.as_str() {
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

			// ── Images ──
			"img" => {
				let src = self.state.image_src.take().unwrap_or_default();
				let alt = self.state.image_alt.take().unwrap_or_default();
				self.push_str("![");
				self.push_str(&alt);
				self.push_str("](");
				self.push_str(&src);
				self.push_char(')');
			}

			// ── Void/self-closing ──
			"hr" | "br" => {
				// already handled in visit_element
			}

			_ => {
				if self.state.is_block_element(&name) {
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

		// if inside an <img>, capture text as alt instead of emitting
		if let Some(ref mut alt) = self.state.image_alt {
			alt.push_str(&text);
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




#[cfg(test)]
mod test {
	use super::*;

	/// Parse markdown then render it back via [`MarkdownRenderer`].
	fn roundtrip(md: &[u8]) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MarkdownParser::new()
			.parse(&mut world.entity_mut(entity), md, None)
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				let mut render = MarkdownRenderer::new();
				walker.walk(&mut render, entity);
				render.into_string()
			})
			.unwrap()
	}

	/// Parse markdown then render with expression support.
	#[allow(dead_code)]
	fn roundtrip_expressions(md: &[u8]) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MarkdownParser::with_expressions()
			.parse(&mut world.entity_mut(entity), md, None)
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				let mut render = MarkdownRenderer::new().with_expressions();
				walker.walk(&mut render, entity);
				render.into_string()
			})
			.unwrap()
	}

	fn trim(input: String) -> String { input.trim().to_string() }

	#[test]
	fn render_paragraph() {
		trim(roundtrip(b"Hello world")).xpect_eq("Hello world");
	}

	#[test]
	fn render_heading_h1() { trim(roundtrip(b"# Title")).xpect_eq("# Title"); }

	#[test]
	fn render_heading_h2() {
		trim(roundtrip(b"## Subtitle")).xpect_eq("## Subtitle");
	}

	#[test]
	fn render_emphasis() { trim(roundtrip(b"*hello*")).xpect_eq("*hello*"); }

	#[test]
	fn render_strong() { trim(roundtrip(b"**hello**")).xpect_eq("**hello**"); }

	#[test]
	fn render_link() {
		trim(roundtrip(b"[click](https://example.com)"))
			.xpect_eq("[click](https://example.com)");
	}

	#[test]
	fn render_image() {
		trim(roundtrip(b"![alt](image.png)")).xpect_eq("![alt](image.png)");
	}

	#[test]
	fn render_unordered_list() {
		trim(roundtrip(b"- alpha\n- beta")).xpect_eq("- alpha\n- beta");
	}

	#[test]
	fn render_code_block() {
		trim(roundtrip(b"```rust\nfn main() {}\n```"))
			.xpect_eq("```rust\nfn main() {}\n```");
	}

	#[test]
	fn render_inline_code() {
		trim(roundtrip(b"use `foo()` here")).xpect_eq("use `foo()` here");
	}

	#[test]
	fn render_blockquote() {
		trim(roundtrip(b"> quoted text")).xpect_eq("> quoted text");
	}

	#[test]
	fn render_blockquote_with_emphasis() {
		// inline elements inside a blockquote must appear after the prefix
		trim(roundtrip(b"> *notable remark*")).xpect_eq("> *notable remark*");
	}

	#[test]
	fn render_blockquote_multiline() {
		// a blockquote whose content spans multiple paragraphs should prefix
		// every paragraph with "> "
		let input = b"> first paragraph\n>\n> second paragraph";
		trim(roundtrip(input))
			.xpect_eq("> first paragraph\n\n> second paragraph");
	}

	#[test]
	fn render_thematic_break() { trim(roundtrip(b"---")).xpect_eq("---"); }

	#[test]
	fn render_multiple_blocks() {
		trim(roundtrip(b"# Title\n\nParagraph"))
			.xpect_eq("# Title\n\nParagraph");
	}

	#[test]
	fn render_comment() {
		trim(roundtrip(b"<!-- hello -->")).xpect_eq("<!-- hello -->");
	}
}
