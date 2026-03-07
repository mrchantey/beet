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
	buffer: String,
	/// Render [`Expression`] values verbatim as `{expr}` in output.
	render_expressions: bool,
	/// Elements that produce block-level output with trailing newlines.
	block_elements: Vec<Cow<'static, str>>,
	/// Track nested list depth for indentation.
	list_depth: usize,
	/// Stack of active inline wrappers to emit on leave.
	inline_stack: Vec<InlineWrapper>,
	/// Stack tracking list item contexts for ordered list numbering.
	list_stack: Vec<ListContext>,
	/// Whether we are inside a `<pre>` block (suppresses markdown formatting).
	in_preformatted: bool,
	/// Pending prefix for the next text node, ie heading markers, list bullets.
	pending_prefix: Option<String>,
	/// Whether a blank line is needed before the next block element.
	needs_block_separator: bool,
	/// Tracks whether we just wrote a newline to avoid doubling.
	trailing_newline: bool,
	/// The href captured from the most recent `<a>` element.
	pending_link_href: Option<String>,
	/// When inside an `<img>`, the src URL from its attribute.
	/// Child text nodes are captured as alt text instead of being emitted.
	image_src: Option<String>,
	/// Accumulated alt text while inside an `<img>` element.
	image_alt: Option<String>,
	/// Code fence info string from `<code>` inside `<pre>`.
	code_fence_info: Option<String>,
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

#[derive(Debug, Clone)]
enum ListContext {
	Unordered,
	Ordered(usize),
}

impl Default for MarkdownRenderer {
	fn default() -> Self { Self::new() }
}

impl MarkdownRenderer {
	pub fn new() -> Self {
		Self {
			buffer: String::new(),
			render_expressions: false,
			block_elements: default_block_elements(),
			list_depth: 0,
			inline_stack: Vec::new(),
			list_stack: Vec::new(),
			in_preformatted: false,
			pending_prefix: None,
			needs_block_separator: false,
			trailing_newline: true, // start of document counts as newline
			pending_link_href: None,
			image_src: None,
			image_alt: None,
			code_fence_info: None,
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
		self.block_elements = elements;
		self
	}

	/// Consume the renderer and return the accumulated markdown string.
	pub fn into_string(self) -> String { self.buffer }

	/// Borrow the accumulated markdown string.
	pub fn as_str(&self) -> &str { &self.buffer }

	fn is_block_element(&self, name: &str) -> bool {
		let lower = name.to_ascii_lowercase();
		self.block_elements.iter().any(|el| el.as_ref() == lower)
	}

	fn ensure_newline(&mut self) {
		if !self.trailing_newline {
			self.buffer.push('\n');
			self.trailing_newline = true;
		}
	}

	fn ensure_block_separator(&mut self) {
		if self.needs_block_separator && !self.buffer.is_empty() {
			self.ensure_newline();
			// add blank line between blocks
			if !self.buffer.ends_with("\n\n") {
				self.buffer.push('\n');
			}
		}
		self.needs_block_separator = false;
	}

	fn push_str(&mut self, text: &str) {
		self.buffer.push_str(text);
		self.trailing_newline = text.ends_with('\n');
	}

	fn push_char(&mut self, ch: char) {
		self.buffer.push(ch);
		self.trailing_newline = ch == '\n';
	}

	fn write_list_indent(&mut self) {
		// indent for nested lists (depth > 1 means we are inside a sub-list)
		if self.list_depth > 1 {
			for _ in 0..self.list_depth - 1 {
				self.push_str("  ");
			}
		}
	}

	fn find_attr<'a>(
		attrs: &'a [(Entity, &Attribute, &Value)],
		key: &str,
	) -> Option<&'a Value> {
		attrs
			.iter()
			.find(|(_, attr, _)| attr.as_str() == key)
			.map(|(_, _, val)| *val)
	}
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
				self.ensure_block_separator();
				let level = name[1..].parse::<usize>().unwrap_or(1);
				let prefix = "#".repeat(level);
				self.pending_prefix = Some(format!("{prefix} "));
			}

			// ── Paragraph ──
			"p" => {
				self.ensure_block_separator();
			}

			// ── Blockquote ──
			"blockquote" => {
				self.ensure_block_separator();
				self.pending_prefix = Some("> ".to_string());
			}

			// ── Lists ──
			"ul" => {
				if self.list_depth == 0 {
					self.ensure_block_separator();
				}
				self.list_depth += 1;
				self.list_stack.push(ListContext::Unordered);
			}
			"ol" => {
				if self.list_depth == 0 {
					self.ensure_block_separator();
				}
				self.list_depth += 1;
				let start = Self::find_attr(&attrs, "start")
					.and_then(|val| match val {
						Value::Uint(num) => Some(*num as usize),
						Value::Int(num) => Some(*num as usize),
						_ => None,
					})
					.unwrap_or(1);
				self.list_stack.push(ListContext::Ordered(start));
			}
			"li" => {
				self.ensure_newline();
				self.write_list_indent();
				let prefix = match self.list_stack.last_mut() {
					Some(ListContext::Unordered) => "- ".to_string(),
					Some(ListContext::Ordered(num)) => {
						let prefix = format!("{}. ", num);
						*num += 1;
						prefix
					}
					None => "- ".to_string(),
				};
				self.push_str(&prefix);
			}

			// ── Code blocks ──
			"pre" => {
				self.ensure_block_separator();
				self.in_preformatted = true;
			}
			"code" if self.in_preformatted => {
				// fenced code block: extract language from class
				let info = Self::find_attr(&attrs, "class")
					.and_then(|val| match val {
						Value::Str(class) => {
							// pulldown-cmark uses "language-rust" style classes
							class
								.strip_prefix("language-")
								.map(|lang| lang.to_string())
								.or_else(|| Some(class.clone()))
						}
						_ => None,
					})
					.unwrap_or_default();
				self.code_fence_info = Some(info.clone());
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
				let href = Self::find_attr(&attrs, "href")
					.map(|val| val.to_string())
					.unwrap_or_default();
				self.pending_link_href = Some(href);
				self.push_char('[');
				self.inline_stack.push(InlineWrapper::Link);
			}

			// ── Images ──
			"img" => {
				let src = Self::find_attr(&attrs, "src")
					.map(|val| val.to_string())
					.unwrap_or_default();
				self.image_src = Some(src);
				self.image_alt = Some(String::new());
			}

			// ── Thematic break ──
			"hr" => {
				self.ensure_block_separator();
				self.push_str("---");
				self.ensure_newline();
				self.needs_block_separator = true;
			}

			// ── Line break ──
			"br" => {
				self.push_str("  \n");
			}

			// ── Tables ──
			// Tables are complex; fall through to raw rendering for now.
			// A full table renderer could be added here.
			"table" | "thead" | "tbody" | "tr" | "th" | "td" => {
				// For tables, we don't add markdown wrappers.
				// The text content flows through visit_value.
			}

			// ── Catch-all for unknown block/inline elements ──
			_ => {
				if self.is_block_element(&name) {
					self.ensure_block_separator();
				}
			}
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		let name = element.name().to_ascii_lowercase();

		match name.as_str() {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				self.ensure_newline();
				self.needs_block_separator = true;
			}

			// ── Paragraph ──
			"p" => {
				self.ensure_newline();
				self.needs_block_separator = true;
			}

			// ── Blockquote ──
			"blockquote" => {
				self.ensure_newline();
				self.needs_block_separator = true;
			}

			// ── Lists ──
			"ul" | "ol" => {
				self.list_depth = self.list_depth.saturating_sub(1);
				self.list_stack.pop();
				if self.list_depth == 0 {
					self.ensure_newline();
					self.needs_block_separator = true;
				}
			}
			"li" => {
				self.ensure_newline();
			}

			// ── Code blocks ──
			"code" if self.in_preformatted => {
				// close fenced code block
				self.ensure_newline();
				self.push_str("```");
				self.push_char('\n');
				self.code_fence_info = None;
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
				self.in_preformatted = false;
				self.needs_block_separator = true;
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
				let href = self.pending_link_href.take().unwrap_or_default();
				self.push_str("](");
				self.push_str(&href);
				self.push_char(')');
			}

			// ── Images ──
			"img" => {
				let src = self.image_src.take().unwrap_or_default();
				let alt = self.image_alt.take().unwrap_or_default();
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
				if self.is_block_element(&name) {
					self.ensure_newline();
					self.needs_block_separator = true;
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
		if let Some(ref mut alt) = self.image_alt {
			alt.push_str(&text);
			return;
		}

		// emit any pending prefix (heading marker, blockquote, etc.)
		if let Some(prefix) = self.pending_prefix.take() {
			self.ensure_newline();
			self.push_str(&prefix);
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
		self.ensure_block_separator();
		self.push_str("<!--");
		self.push_str(comment);
		self.push_str("-->");
		self.ensure_newline();
		self.needs_block_separator = true;
	}
}




#[cfg(test)]
mod test {
	use super::*;

	/// Parse markdown then render it back via [`MarkdownRenderer`].
	fn roundtrip(md: &[u8]) -> String {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		MarkdownParser::new()
			.parse(&mut world, entity, md.to_vec(), None)
			.unwrap();
		let mut renderer = Some(MarkdownRenderer::new());
		world
			.run_system_once(move |walker: NodeWalker| {
				let mut render = renderer.take().unwrap();
				walker.walk(&mut render, entity);
				render.into_string()
			})
			.unwrap()
	}

	/// Parse markdown then render with expression support.
	#[allow(dead_code)]
	fn roundtrip_expressions(md: &[u8]) -> String {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		MarkdownParser::with_expressions()
			.parse(&mut world, entity, md.to_vec(), None)
			.unwrap();
		let mut renderer = Some(MarkdownRenderer::new().with_expressions());
		world
			.run_system_once(move |walker: NodeWalker| {
				let mut render = renderer.take().unwrap();
				walker.walk(&mut render, entity);
				render.into_string()
			})
			.unwrap()
	}

	fn trim(input: String) -> String { input.trim().to_string() }

	#[test]
	fn render_paragraph() {
		trim(roundtrip(b"Hello world")).xpect_eq("Hello world".to_string());
	}

	#[test]
	fn render_heading_h1() {
		trim(roundtrip(b"# Title")).xpect_eq("# Title".to_string());
	}

	#[test]
	fn render_heading_h2() {
		trim(roundtrip(b"## Subtitle")).xpect_eq("## Subtitle".to_string());
	}

	#[test]
	fn render_emphasis() {
		trim(roundtrip(b"*hello*")).xpect_contains("*hello*");
	}

	#[test]
	fn render_strong() {
		trim(roundtrip(b"**hello**")).xpect_contains("**hello**");
	}

	#[test]
	fn render_link() {
		let result = trim(roundtrip(b"[click](https://example.com)"));
		result
			.xpect_contains("[click]")
			.xpect_contains("(https://example.com)");
	}

	#[test]
	fn render_image() {
		let result = trim(roundtrip(b"![alt](image.png)"));
		result
			.xpect_contains("![alt]")
			.xpect_contains("(image.png)");
	}

	#[test]
	fn render_unordered_list() {
		let result = trim(roundtrip(b"- a\n- b"));
		result.xpect_contains("- a").xpect_contains("- b");
	}

	#[test]
	fn render_code_block() {
		let result = trim(roundtrip(b"```rust\nfn main() {}\n```"));
		result
			.xpect_contains("```rust")
			.xpect_contains("fn main() {}")
			.xpect_contains("```");
	}

	#[test]
	fn render_inline_code() {
		trim(roundtrip(b"use `foo()` here")).xpect_contains("`foo()`");
	}

	#[test]
	fn render_blockquote() {
		trim(roundtrip(b"> quoted text")).xpect_contains("> quoted text");
	}

	#[test]
	fn render_thematic_break() { roundtrip(b"---").xpect_contains("---"); }

	#[test]
	fn render_multiple_blocks() {
		let result = trim(roundtrip(b"# Title\n\nParagraph"));
		result.xpect_contains("# Title").xpect_contains("Paragraph");
	}

	#[test]
	fn render_comment() {
		let result = trim(roundtrip(b"<!-- hello -->"));
		result.xpect_contains("<!-- hello -->");
	}
}
