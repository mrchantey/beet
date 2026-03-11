use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;

/// Renders an entity tree back to a markdown string via [`NodeVisitor`].
///
/// Converts HTML-like element trees (as produced by [`MarkdownParser`])
/// back into CommonMark-compatible markdown. Supports headings, emphasis,
/// strong, links, images, lists, code blocks, blockquotes, thematic
/// breaks, inline code, and optional expression rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
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
	fn visit_element(&mut self, _cx: &VisitContext, view: &ElementView) {
		let name = view.name();

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
				let start = view.ol_start();
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

			// ── Tables ──
			"table" | "thead" | "tbody" | "tr" | "th" | "td" => {
				// text content flows through visit_value
			}

			// ── Catch-all for unknown block/inline elements ──
			_ => {
				if self.state.is_block_element(name) {
					self.state.ensure_block_separator();
				}
			}
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		let name = element.name();

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
		cx: &RenderContext,
	) -> Result<RenderOutput, RenderError> {
		cx.check_accepts(&[MediaType::Markdown])?;
		cx.walker.walk(self, cx.entity);
		RenderOutput::media_string(
			MediaType::Markdown,
			std::mem::take(&mut self.state.buffer),
		)
		.xok()
	}
}


#[cfg(test)]
mod test {
	use super::*;

	/// Parse markdown then render it back via [`MarkdownRenderer`].
	fn roundtrip(md: &str) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::markdown(md);
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				MarkdownRenderer::new()
					.render(&RenderContext::new(entity, &walker))
					.unwrap()
					.to_string()
			})
			.unwrap()
	}

	/// Parse markdown then render with expression support.
	#[allow(dead_code)]
	fn roundtrip_expressions(md: &str) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::markdown(md);
		MarkdownParser::with_expressions()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		world
			.run_system_once(move |walker: NodeWalker| {
				MarkdownRenderer::new()
					.with_expressions()
					.render(&RenderContext::new(entity, &walker))
					.unwrap()
					.to_string()
			})
			.unwrap()
	}

	#[test]
	fn render_paragraph() {
		roundtrip("Hello world").trim().xpect_eq("Hello world");
	}

	#[test]
	fn render_heading_h1() { roundtrip("# Title").trim().xpect_eq("# Title"); }

	#[test]
	fn render_heading_h2() {
		roundtrip("## Subtitle").trim().xpect_eq("## Subtitle");
	}

	#[test]
	fn render_emphasis() { roundtrip("*hello*").trim().xpect_eq("*hello*"); }

	#[test]
	fn render_strong() { roundtrip("**hello**").trim().xpect_eq("**hello**"); }

	#[test]
	fn render_link() {
		roundtrip("[click](https://example.com)")
			.trim()
			.xpect_eq("[click](https://example.com)");
	}

	#[test]
	fn render_image() {
		roundtrip("![alt](image.png)")
			.trim()
			.xpect_eq("![alt](image.png)");
	}

	#[test]
	fn render_unordered_list() {
		roundtrip("- alpha\n- beta")
			.trim()
			.xpect_eq("- alpha\n- beta");
	}

	#[test]
	fn render_code_block() {
		roundtrip("```rust\nfn main() {}\n```")
			.trim()
			.xpect_eq("```rust\nfn main() {}\n```");
	}

	#[test]
	fn render_inline_code() {
		roundtrip("use `foo()` here")
			.trim()
			.xpect_eq("use `foo()` here");
	}

	#[test]
	fn render_blockquote() {
		roundtrip("> quoted text").trim().xpect_eq("> quoted text");
	}

	#[test]
	fn render_blockquote_with_emphasis() {
		// inline elements inside a blockquote must appear after the prefix
		roundtrip("> *notable remark*")
			.trim()
			.xpect_eq("> *notable remark*");
	}

	#[test]
	fn render_blockquote_multiline() {
		let input = "> first paragraph\n>\n> second paragraph";
		roundtrip(input).trim().xpect_eq(input);
	}

	#[test]
	fn render_thematic_break() { roundtrip("---").trim().xpect_eq("---"); }

	#[test]
	fn render_multiple_blocks() {
		roundtrip("# Title\n\nParagraph")
			.trim()
			.xpect_eq("# Title\n\nParagraph");
	}

	#[test]
	fn render_comment() {
		roundtrip("<!-- hello -->")
			.trim()
			.xpect_eq("<!-- hello -->");
	}
}
