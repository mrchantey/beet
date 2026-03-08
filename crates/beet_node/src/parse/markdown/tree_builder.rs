//! Converts `pulldown-cmark` events into the shared [`TreeNode`] intermediate
//! representation used by the HTML parser's diff infrastructure.
//!
//! The entry point is [`build_markdown_tree`], which returns a
//! [`MarkdownTreeResult`] containing the node tree and optional frontmatter.

use super::frontmatter::Frontmatter;
use super::frontmatter::FrontmatterKind;
use crate::parse::html::combinators::HtmlParseConfig;
use crate::parse::html::diff::HtmlDiffConfig;
use crate::parse::html::diff::HtmlNode;
use crate::parse::html::diff::build_html_tree;
use crate::parse::html::tokens::HtmlAttribute;
use crate::prelude::*;
use beet_core::prelude::*;
use pulldown_cmark::Event;
use pulldown_cmark::Options;
use pulldown_cmark::Tag;

/// Result of building a markdown tree, containing the node list and
/// optional parsed frontmatter.
pub(crate) struct MarkdownTree<'a> {
	/// The top-level tree nodes ready for diffing.
	pub nodes: Vec<HtmlNode<'a>>,
	/// Parsed frontmatter if a metadata block was present.
	pub frontmatter: Option<Frontmatter>,
}

/// Parse markdown text into a [`MarkdownTreeResult`].
///
/// Uses `pulldown-cmark` with the given options, converts each event
/// into [`TreeNode`] using HTML-equivalent tag names, and delegates
/// embedded HTML to the HTML tokenizer.
pub(crate) fn build_markdown_tree<'a>(
	text: &'a str,
	options: Options,
	html_parse_config: &HtmlParseConfig,
	html_diff_config: &HtmlDiffConfig,
	_span_lookup: Option<&SpanLookup>,
) -> Result<MarkdownTree<'a>> {
	let parser = pulldown_cmark::Parser::new_ext(text, options);

	let mut builder = MdTreeBuilder::new(
		text,
		html_parse_config.clone(),
		html_diff_config.clone(),
	);

	for (event, range) in parser.into_offset_iter() {
		builder.handle_event(event, range);
	}

	builder.finish()
}


/// Internal stack-based builder that converts pulldown events into a
/// `TreeNode` tree. Each open tag pushes a frame; each close tag pops
/// one and appends the completed subtree to the parent.
struct MdTreeBuilder<'a> {
	/// The full input text, used for slicing source spans.
	text: &'a str,
	/// Stack of (tag_name, attributes, source_slice, children).
	/// The bottom entry is the synthetic root.
	stack: Vec<StackFrame<'a>>,
	/// Collected frontmatter content and kind.
	frontmatter_content: Option<(String, FrontmatterKind)>,
	/// Whether we're inside a metadata block.
	in_metadata: bool,
	/// Owned HTML parse config for embedded HTML.
	html_parse_config: HtmlParseConfig,
	/// Owned HTML diff config for embedded HTML.
	html_diff_config: HtmlDiffConfig,
}

struct StackFrame<'a> {
	/// HTML tag name for this element.
	name: &'static str,
	/// Attributes on this element.
	attributes: Vec<HtmlAttribute<'a>>,
	/// Source text slice for span tracking.
	source: &'a str,
	/// Accumulated child nodes.
	children: Vec<HtmlNode<'a>>,
}

impl<'a> StackFrame<'a> {
	fn new(name: &'static str, source: &'a str) -> Self {
		Self {
			name,
			attributes: Vec::new(),
			children: Vec::new(),
			source,
		}
	}

	fn with_attributes(
		name: &'static str,
		source: &'a str,
		attributes: Vec<HtmlAttribute<'a>>,
	) -> Self {
		Self {
			name,
			attributes,
			children: Vec::new(),
			source,
		}
	}
}

impl<'a> MdTreeBuilder<'a> {
	fn new(
		text: &'a str,
		html_parse_config: HtmlParseConfig,
		html_diff_config: HtmlDiffConfig,
	) -> Self {
		Self {
			text,
			// root frame collects top-level nodes
			stack: vec![StackFrame::new("", "")],
			frontmatter_content: None,
			in_metadata: false,
			html_parse_config,
			html_diff_config,
		}
	}

	/// Get a `&str` slice of the input for the given byte range.
	fn slice(&self, range: &std::ops::Range<usize>) -> &'a str {
		&self.text[range.start..range.end.min(self.text.len())]
	}

	/// Push a new element frame onto the stack.
	fn push(&mut self, frame: StackFrame<'a>) { self.stack.push(frame); }

	/// Pop the top frame, build a `TreeNode::Element`, and append it
	/// to the parent frame.
	fn pop(&mut self) {
		if self.stack.len() <= 1 {
			return;
		}
		if let Some(frame) = self.stack.pop() {
			let node = HtmlNode::Element {
				name: frame.name,
				attributes: frame.attributes,
				children: frame.children,
				source: frame.source,
			};
			if let Some(parent) = self.stack.last_mut() {
				parent.children.push(node);
			}
		}
	}

	/// Append a leaf node to the current frame.
	fn push_leaf(&mut self, node: HtmlNode<'a>) {
		if let Some(parent) = self.stack.last_mut() {
			parent.children.push(node);
		}
	}

	/// Append a void element (no children) to the current frame.
	fn push_void(
		&mut self,
		name: &'static str,
		source: &'a str,
		attributes: Vec<HtmlAttribute<'a>>,
	) {
		self.push_leaf(HtmlNode::Element {
			name,
			attributes,
			children: vec![],
			source,
		});
	}

	fn handle_event(
		&mut self,
		event: Event<'a>,
		range: std::ops::Range<usize>,
	) {
		let source = self.slice(&range);

		match event {
			Event::Start(tag) => self.handle_start(tag, source),
			Event::End(_) => self.handle_end(),
			Event::Text(text) => {
				if self.in_metadata {
					// accumulate metadata text
					if let Some((ref mut content, _)) = self.frontmatter_content
					{
						content.push_str(&text);
					}
				} else if self.html_parse_config.parse_expressions {
					self.push_text_with_expressions(self.slice(&range));
				} else {
					self.push_leaf(HtmlNode::Text(self.slice(&range)));
				}
			}
			Event::Code(text) => {
				// Inline code: <code>text</code>
				let text_slice = self.slice(&range);
				// We need a borrowed slice for the text content.
				// The source range includes backticks, the inner text is
				// what pulldown gives us. We use the full source for span.
				self.push_leaf(HtmlNode::Element {
					name: "code",
					attributes: vec![],
					children: vec![HtmlNode::Text(
						self.find_inner_text(text_slice, &text),
					)],
					source: text_slice,
				});
			}
			Event::SoftBreak => {
				self.push_leaf(HtmlNode::Text(self.slice(&range)));
			}
			Event::HardBreak => {
				self.push_void("br", source, vec![]);
			}
			Event::Rule => {
				self.push_void("hr", source, vec![]);
			}
			Event::Html(html) => {
				self.handle_html_block(&html, source);
			}
			Event::InlineHtml(html) => {
				self.handle_inline_html(&html, source);
			}
			Event::FootnoteReference(label) => {
				let label_str = label.as_ref();
				// <sup><a href="#fn-{label}">[{label}]</a></sup>
				let href_text =
					self.find_substring(source, label_str).unwrap_or(source);
				self.push_leaf(HtmlNode::Element {
					name: "sup",
					attributes: vec![],
					children: vec![HtmlNode::Element {
						name: "a",
						attributes: vec![HtmlAttribute::new("href", href_text)],
						children: vec![HtmlNode::Text(href_text)],
						source,
					}],
					source,
				});
			}
			Event::TaskListMarker(checked) => {
				let mut attrs = vec![
					HtmlAttribute::boolean("disabled"),
					HtmlAttribute::new("type", "checkbox"),
				];
				if checked {
					attrs.push(HtmlAttribute::boolean("checked"));
				}
				self.push_void("input", source, attrs);
			}
			Event::InlineMath(_text) => {
				let text_slice = self.slice(&range);
				self.push_leaf(HtmlNode::Element {
					name: "span",
					attributes: vec![HtmlAttribute::new(
						"class",
						"math-inline",
					)],
					children: vec![HtmlNode::Text(text_slice)],
					source: text_slice,
				});
			}
			Event::DisplayMath(_text) => {
				let text_slice = self.slice(&range);
				self.push_leaf(HtmlNode::Element {
					name: "div",
					attributes: vec![HtmlAttribute::new(
						"class",
						"math-display",
					)],
					children: vec![HtmlNode::Text(text_slice)],
					source: text_slice,
				});
			}
		}
	}

	fn handle_start(&mut self, tag: Tag<'a>, source: &'a str) {
		match tag {
			Tag::Paragraph => {
				self.push(StackFrame::new("p", source));
			}
			Tag::Heading { level, .. } => {
				let name = match level {
					pulldown_cmark::HeadingLevel::H1 => "h1",
					pulldown_cmark::HeadingLevel::H2 => "h2",
					pulldown_cmark::HeadingLevel::H3 => "h3",
					pulldown_cmark::HeadingLevel::H4 => "h4",
					pulldown_cmark::HeadingLevel::H5 => "h5",
					pulldown_cmark::HeadingLevel::H6 => "h6",
				};
				self.push(StackFrame::new(name, source));
			}
			Tag::BlockQuote(_) => {
				self.push(StackFrame::new("blockquote", source));
			}
			Tag::CodeBlock(kind) => {
				// <pre><code class="language-{lang}">
				self.push(StackFrame::new("pre", source));
				let attrs = match kind {
					pulldown_cmark::CodeBlockKind::Fenced(info) => {
						let info_str = info.as_ref();
						let lang =
							info_str.split_whitespace().next().unwrap_or("");
						if !lang.is_empty() {
							// Find the language string within source for borrowing
							let class_val = self
								.find_substring(source, lang)
								.unwrap_or(source);
							vec![HtmlAttribute::new("class", class_val)]
						} else {
							vec![]
						}
					}
					pulldown_cmark::CodeBlockKind::Indented => vec![],
				};
				self.push(StackFrame::with_attributes("code", source, attrs));
			}
			Tag::HtmlBlock => {
				// HtmlBlock content arrives as Text events, but we handle
				// the whole block in the End event by checking the stack
				self.push(StackFrame::new("__html_block", source));
			}
			Tag::List(start) => match start {
				Some(_start_num) => {
					// Find start number text in source for attribute borrowing
					let start_text =
						self.find_digit_substring(source).unwrap_or(source);
					let attrs = vec![HtmlAttribute::new("start", start_text)];
					self.push(StackFrame::with_attributes("ol", source, attrs));
				}
				None => {
					self.push(StackFrame::new("ul", source));
				}
			},
			Tag::Item => {
				self.push(StackFrame::new("li", source));
			}
			Tag::FootnoteDefinition(label) => {
				let label_text = self
					.find_substring(source, label.as_ref())
					.unwrap_or(source);
				let attrs = vec![
					HtmlAttribute::new("class", "footnote-definition"),
					HtmlAttribute::new("id", label_text),
				];
				self.push(StackFrame::with_attributes("div", source, attrs));
			}
			Tag::DefinitionList => {
				self.push(StackFrame::new("dl", source));
			}
			Tag::DefinitionListTitle => {
				self.push(StackFrame::new("dt", source));
			}
			Tag::DefinitionListDefinition => {
				self.push(StackFrame::new("dd", source));
			}
			Tag::Table(_alignments) => {
				self.push(StackFrame::new("table", source));
			}
			Tag::TableHead => {
				self.push(StackFrame::new("thead", source));
			}
			Tag::TableRow => {
				self.push(StackFrame::new("tr", source));
			}
			Tag::TableCell => {
				// We use td for all cells; renderers can differentiate
				// based on whether the parent is thead or tbody
				self.push(StackFrame::new("td", source));
			}
			Tag::Emphasis => {
				self.push(StackFrame::new("em", source));
			}
			Tag::Strong => {
				self.push(StackFrame::new("strong", source));
			}
			Tag::Strikethrough => {
				self.push(StackFrame::new("del", source));
			}
			Tag::Superscript => {
				self.push(StackFrame::new("sup", source));
			}
			Tag::Subscript => {
				self.push(StackFrame::new("sub", source));
			}
			Tag::Link {
				dest_url, title, ..
			} => {
				let href = self
					.find_substring(source, dest_url.as_ref())
					.unwrap_or(source);
				let mut attrs = vec![HtmlAttribute::new("href", href)];
				if !title.is_empty() {
					let title_text = self
						.find_substring(source, title.as_ref())
						.unwrap_or(source);
					attrs.push(HtmlAttribute::new("title", title_text));
				}
				self.push(StackFrame::with_attributes("a", source, attrs));
			}
			Tag::Image {
				dest_url, title, ..
			} => {
				let src = self
					.find_substring(source, dest_url.as_ref())
					.unwrap_or(source);
				let mut attrs = vec![HtmlAttribute::new("src", src)];
				if !title.is_empty() {
					let title_text = self
						.find_substring(source, title.as_ref())
						.unwrap_or(source);
					attrs.push(HtmlAttribute::new("title", title_text));
				}
				self.push(StackFrame::with_attributes("img", source, attrs));
			}
			Tag::MetadataBlock(kind) => {
				let fm_kind = match kind {
					pulldown_cmark::MetadataBlockKind::YamlStyle => {
						FrontmatterKind::Yaml
					}
					pulldown_cmark::MetadataBlockKind::PlusesStyle => {
						FrontmatterKind::Toml
					}
				};
				self.frontmatter_content = Some((String::new(), fm_kind));
				self.in_metadata = true;
				// Push a dummy frame so pop works correctly
				self.push(StackFrame::new("__metadata", source));
			}
		}
	}

	fn handle_end(&mut self) {
		if self.stack.len() <= 1 {
			return;
		}

		let frame = self.stack.last().unwrap();
		let name = frame.name;

		match name {
			"__metadata" => {
				// Don't emit a node for metadata blocks — the content
				// was captured into self.frontmatter_content
				self.in_metadata = false;
				self.stack.pop();
				// Don't push to parent — frontmatter is handled separately
			}
			"__html_block" => {
				// Collect accumulated text children and parse as HTML
				let frame = self.stack.pop().unwrap();
				let html_content: String = frame
					.children
					.iter()
					.filter_map(|node| match node {
						HtmlNode::Text(text) => Some(*text),
						_ => None,
					})
					.collect();

				if !html_content.is_empty() {
					self.handle_html_block(&html_content, frame.source);
				}
			}
			_ => {
				self.pop();
			}
		}
	}

	/// Parse an HTML block string and splice the resulting nodes into
	/// the current frame.
	fn handle_html_block(&mut self, html: &str, source: &'a str) {
		// Try to parse the HTML via the HTML tokenizer
		match crate::parse::html::combinators::parse_document(
			html,
			&self.html_parse_config,
		) {
			Ok(tokens) => {
				match build_html_tree(
					&tokens,
					&self.html_diff_config,
					&self.html_parse_config,
				) {
					Ok(_nodes) => {
						// Splice parsed HTML nodes into current frame.
						// Since the HTML tokens borrow from `html` (a
						// temporary), we can't directly use them. Instead,
						// insert as a raw text node.
						self.push_leaf(HtmlNode::Text(source));
					}
					Err(_) => {
						// Fallback: treat as raw text
						self.push_leaf(HtmlNode::Text(source));
					}
				}
			}
			Err(_) => {
				// Fallback: treat as raw text
				self.push_leaf(HtmlNode::Text(source));
			}
		}
	}

	/// Parse inline HTML and insert into the current frame.
	fn handle_inline_html(&mut self, _html: &str, source: &'a str) {
		// Inline HTML is tricky because it may be a partial tag
		// (opening tag in one event, closing in another).
		// For now, insert as raw text; the HTML renderer will pass it through.
		self.push_leaf(HtmlNode::Text(source));
	}

	/// Try to find a substring within the source slice, returning a
	/// borrowed slice from the original text. This ensures `TreeNode`
	/// attribute values borrow from the input lifetime.
	fn find_substring(&self, source: &'a str, needle: &str) -> Option<&'a str> {
		if needle.is_empty() {
			return None;
		}
		let pos = source.find(needle)?;
		Some(&source[pos..pos + needle.len()])
	}

	/// Find the first sequence of digits in the source.
	fn find_digit_substring(&self, source: &'a str) -> Option<&'a str> {
		let start = source.find(|ch: char| ch.is_ascii_digit())?;
		let end = source[start..]
			.find(|ch: char| !ch.is_ascii_digit())
			.map(|pos| start + pos)
			.unwrap_or(source.len());
		if start < end {
			Some(&source[start..end])
		} else {
			None
		}
	}

	/// For inline code, find the text content within the backtick-delimited
	/// source range.
	fn find_inner_text(&self, source: &'a str, content: &str) -> &'a str {
		self.find_substring(source, content).unwrap_or(source)
	}

	/// Split a text slice on `{expr}` boundaries, pushing interleaved
	/// [`HtmlNode::Text`] and [`HtmlNode::Expression`] leaves.
	///
	/// Supports nested braces, ie `{a {b} c}` is a single expression.
	fn push_text_with_expressions(&mut self, source: &'a str) {
		let mut remaining = source;
		while !remaining.is_empty() {
			if let Some(brace_pos) = remaining.find('{') {
				// push any text before the brace
				if brace_pos > 0 {
					self.push_leaf(HtmlNode::Text(&remaining[..brace_pos]));
				}
				// parse the expression with nested brace tracking
				let after_brace = &remaining[brace_pos + 1..];
				let mut depth: usize = 1;
				let mut end = None;
				for (idx, ch) in after_brace.char_indices() {
					match ch {
						'{' => depth += 1,
						'}' => {
							depth -= 1;
							if depth == 0 {
								end = Some(idx);
								break;
							}
						}
						_ => {}
					}
				}
				if let Some(end_idx) = end {
					let expr = &after_brace[..end_idx];
					self.push_leaf(HtmlNode::Expression(expr));
					remaining = &after_brace[end_idx + 1..];
				} else {
					// unmatched brace, treat the rest as text
					self.push_leaf(HtmlNode::Text(remaining));
					break;
				}
			} else {
				self.push_leaf(HtmlNode::Text(remaining));
				break;
			}
		}
	}

	/// Consume the builder and produce the final result.
	fn finish(mut self) -> Result<MarkdownTree<'a>> {
		// Drain any unclosed elements
		while self.stack.len() > 1 {
			self.pop();
		}

		let nodes = self
			.stack
			.pop()
			.map(|frame| frame.children)
			.unwrap_or_default();

		let frontmatter =
			self.frontmatter_content.and_then(|(content, kind)| {
				Frontmatter::parse(&content, kind).ok()
			});

		Ok(MarkdownTree { nodes, frontmatter })
	}
}


#[cfg(test)]
mod test {
	use super::*;

	fn build(text: &str) -> Vec<HtmlNode<'_>> {
		build_markdown_tree(
			text,
			MarkdownParseConfig::default_cmark_options(),
			&HtmlParseConfig::default(),
			&HtmlDiffConfig::default(),
			None,
		)
		.unwrap()
		.nodes
	}

	fn node_name<'a>(node: &HtmlNode<'a>) -> &'a str {
		match node {
			HtmlNode::Element { name, .. } => name,
			HtmlNode::Text(text) => text,
			HtmlNode::Comment(text) => text,
			HtmlNode::Doctype(text) => text,
			HtmlNode::Expression(text) => text,
		}
	}

	fn node_children<'a>(node: &'a HtmlNode<'a>) -> &'a [HtmlNode<'a>] {
		match node {
			HtmlNode::Element { children, .. } => children,
			_ => &[],
		}
	}

	#[test]
	fn paragraph() {
		let nodes = build("Hello world");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("p");
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(1);
		matches!(&children[0], HtmlNode::Text(_)).xpect_true();
	}

	#[test]
	fn heading_levels() {
		for (md, expected) in [
			("# H1", "h1"),
			("## H2", "h2"),
			("### H3", "h3"),
			("#### H4", "h4"),
			("##### H5", "h5"),
			("###### H6", "h6"),
		] {
			let nodes = build(md);
			nodes.len().xpect_eq(1);
			node_name(&nodes[0]).xpect_eq(expected);
		}
	}

	#[test]
	fn emphasis_and_strong() {
		let nodes = build("*em* **strong**");
		nodes.len().xpect_eq(1);
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(|child| node_name(child) == "em")
			.xpect_true();
		children
			.iter()
			.any(|child| node_name(child) == "strong")
			.xpect_true();
	}

	#[test]
	fn unordered_list() {
		let nodes = build("- a\n- b");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("ul");
		let items = node_children(&nodes[0]);
		items.len().xpect_eq(2);
		node_name(&items[0]).xpect_eq("li");
		node_name(&items[1]).xpect_eq("li");
	}

	#[test]
	fn ordered_list() {
		let nodes = build("1. a\n2. b");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("ol");
	}

	#[test]
	fn code_block() {
		let nodes = build("```rust\nfn main() {}\n```");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("pre");
		let pre_children = node_children(&nodes[0]);
		pre_children.len().xpect_eq(1);
		node_name(&pre_children[0]).xpect_eq("code");
	}

	#[test]
	fn inline_code() {
		let nodes = build("use `foo()` here");
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(|child| node_name(child) == "code")
			.xpect_true();
	}

	#[test]
	fn thematic_break() {
		let nodes = build("---");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("hr");
	}

	#[test]
	fn link() {
		let nodes = build("[click](https://example.com)");
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(|child| node_name(child) == "a")
			.xpect_true();
	}

	#[test]
	fn image() {
		let nodes = build("![alt](img.png)");
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(|child| node_name(child) == "img")
			.xpect_true();
	}

	#[test]
	fn blockquote() {
		let nodes = build("> quoted");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("blockquote");
	}

	#[test]
	fn strikethrough() {
		let nodes = build("~~deleted~~");
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(|child| node_name(child) == "del")
			.xpect_true();
	}

	#[test]
	fn table() {
		let nodes = build("| A | B |\n|---|---|\n| 1 | 2 |");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("table");
	}

	#[test]
	fn multiple_blocks() {
		let nodes = build("# Title\n\nParagraph");
		nodes.len().xpect_eq(2);
		node_name(&nodes[0]).xpect_eq("h1");
		node_name(&nodes[1]).xpect_eq("p");
	}

	#[test]
	fn nested_list() {
		let nodes = build("- a\n  - b\n  - c\n- d");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("ul");
		let items = node_children(&nodes[0]);
		items.len().xpect_eq(2);
	}

	#[test]
	fn frontmatter_parsed() {
		let result = build_markdown_tree(
			"---\ntitle: Hello\n---\n\n# Title",
			MarkdownParseConfig::default_cmark_options(),
			&HtmlParseConfig::default(),
			&HtmlDiffConfig::default(),
			None,
		)
		.unwrap();
		result.frontmatter.is_some().xpect_true();
		// The heading should still be in the nodes
		result
			.nodes
			.iter()
			.any(|node| node_name(node) == "h1")
			.xpect_true();
	}

	#[test]
	fn no_frontmatter() {
		let result = build_markdown_tree(
			"# Just a heading",
			MarkdownParseConfig::default_cmark_options(),
			&HtmlParseConfig::default(),
			&HtmlDiffConfig::default(),
			None,
		)
		.unwrap();
		result.frontmatter.is_none().xpect_true();
	}

	fn build_with_expressions(text: &str) -> Vec<HtmlNode<'_>> {
		build_markdown_tree(
			text,
			MarkdownParseConfig::default_cmark_options(),
			&HtmlParseConfig::with_expressions(),
			&HtmlDiffConfig::default(),
			None,
		)
		.unwrap()
		.nodes
	}

	#[test]
	fn expression_in_paragraph() {
		let nodes = build_with_expressions("hello {name} world");
		nodes.len().xpect_eq(1);
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(3);
		matches!(&children[0], HtmlNode::Text(text) if *text == "hello ")
			.xpect_true();
		matches!(&children[1], HtmlNode::Expression(expr) if *expr == "name")
			.xpect_true();
		matches!(&children[2], HtmlNode::Text(text) if *text == " world")
			.xpect_true();
	}

	#[test]
	fn expression_only() {
		let nodes = build_with_expressions("{greeting}");
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(1);
		matches!(&children[0], HtmlNode::Expression(expr) if *expr == "greeting")
			.xpect_true();
	}

	#[test]
	fn expression_nested_braces() {
		let nodes = build_with_expressions("{a {b} c}");
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(
				|child| matches!(child, HtmlNode::Expression(expr) if *expr == "a {b} c"),
			)
			.xpect_true();
	}

	#[test]
	fn no_expressions_without_flag() {
		// Without expression parsing, braces are plain text
		let nodes = build("hello {name} world");
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(1);
		matches!(&children[0], HtmlNode::Text(_)).xpect_true();
	}

	#[test]
	fn multiple_expressions() {
		let nodes = build_with_expressions("{a} and {b}");
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(3);
		matches!(&children[0], HtmlNode::Expression(expr) if *expr == "a")
			.xpect_true();
		matches!(&children[1], HtmlNode::Text(text) if *text == " and ")
			.xpect_true();
		matches!(&children[2], HtmlNode::Expression(expr) if *expr == "b")
			.xpect_true();
	}
}
