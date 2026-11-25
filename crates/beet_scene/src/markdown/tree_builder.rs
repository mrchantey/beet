//! Converts `pulldown-cmark` events into the [`BsxNode`] tree every markup
//! format lowers from.
//!
//! This is the markdown structure logic: a stack-based builder that interleaves
//! prose with embedded markup. Each embedded HTML fragment is parsed by the core
//! BSX fragment primitive ([`BsxFragmentToken::parse_fragment`]) and applied to the shared stack,
//! so a single element can span several `pulldown-cmark` events (the open tag on
//! one line, the close on another) and a paired inline tag can wrap the markdown
//! text between its open and close.
//!
//! The entry point is [`MarkdownTree::build`].

use super::MarkdownParser;
use crate::prelude::*;
use beet_core::prelude::*;
use core::ops::Range;
use pulldown_cmark::CodeBlockKind;
use pulldown_cmark::Event;
use pulldown_cmark::Tag;
use pulldown_cmark::TagEnd;

/// A markdown document as markup nodes, and its frontmatter.
pub(crate) struct MarkdownTree {
	/// The top-level nodes.
	pub nodes: Vec<BsxNode>,
	/// Parsed frontmatter if a metadata block was present.
	pub frontmatter: Option<Frontmatter>,
}

impl MarkdownTree {
	/// Parse markdown text with `pulldown-cmark`, converting each event into a
	/// [`BsxNode`] using HTML-equivalent tag names, and delegating embedded
	/// markup to the BSX fragment primitive
	/// ([`BsxFragmentToken::parse_fragment`]).
	///
	/// A fence whose info string names a media type one of `parsers` reads is
	/// that document, embedded.
	pub fn build(
		text: &str,
		parser: &MarkdownParser,
		parsers: &DocParsers,
	) -> Result<Self> {
		let mut builder =
			MdTreeBuilder::new(text, parser.fragment.clone(), parsers);
		for (event, range) in
			pulldown_cmark::Parser::new_ext(text, parser.config.options)
				.into_offset_iter()
		{
			builder.handle_event(event, range)?;
		}
		builder.finish()
	}
}

/// Build a plain string attribute `key="value"`.
fn str_attr(key: &str, value: &str) -> BsxAttribute {
	BsxAttribute::new(key, AttrValue::Str(value.to_string()))
}

/// Build a bare flag attribute, ie `disabled`.
fn flag_attr(key: &str) -> BsxAttribute {
	BsxAttribute::new(key, AttrValue::Flag)
}

/// An element node written at `span`.
fn element(
	tag: impl Into<String>,
	attributes: Vec<BsxAttribute>,
	children: Vec<BsxNode>,
	span: impl Into<SourceSpan>,
) -> BsxNode {
	BsxNode::Element(BsxElement::new(tag, attributes, children).with_span(span))
}

/// A text node written at `span`, its entities unescaped.
fn text_node(text: &str, span: impl Into<SourceSpan>) -> BsxNode {
	BsxNode::Text(Spanned::new(html_ext::unescape_text(text), span))
}

/// Whether `tag` names an HTML void element. A capitalized component tag (the
/// `<Link>` widget) never matches the lowercase void it shares letters with, so
/// it keeps its children.
fn is_void_element(tag: &str) -> bool { html_ext::VOID_ELEMENTS.contains(&tag) }

/// Whether the node is whitespace-only text, ie carries no visible content.
fn is_blank_text(node: &BsxNode) -> bool {
	matches!(node, BsxNode::Text(text) if text.trim().is_empty())
}

/// Whether the node is a block-level element (see [`html_ext::is_block_element`]). Text
/// and non-visual nodes (comments, doctype, expressions) are not block boxes, so
/// they join the surrounding inline run.
fn is_block_level(node: &BsxNode) -> bool {
	match node {
		BsxNode::Element(element) => html_ext::is_block_element(&element.tag),
		// a fence is a block
		BsxNode::Embedded(_) => true,
		_ => false,
	}
}

/// Drop insignificant inter-element whitespace from an embedded-markup element's
/// children, the markdown-path twin of the BSX parser's `normalize_whitespace` (and
/// of the `rsx!` macro's compile-time trim). Embedded markup like
/// `<div class="design-row">\n\t<Button/>\n\t<Button/>\n</div>` carries the
/// formatting indentation as whitespace-only text nodes; the browser collapses
/// them but the charcell layout would render each as a blank row / extra gap, so
/// they must be dropped before the tree is built (markdown-native content has no
/// such nodes, so this is a no-op there).
///
/// A whitespace-only [`BsxNode::Text`] is insignificant when neither neighbour
/// (nor a list edge) is inline content (non-blank text or an `{expr}`): an
/// *element* neighbour, even an inline one like `<button>`, does not make it
/// significant, matching how `rsx!` trims between tags. Whitespace touching real
/// text/`{expr}` stays (collapsing to one space at render).
fn drop_insignificant_whitespace(children: &mut Vec<BsxNode>) {
	let is_inline = |node: &BsxNode| {
		matches!(node, BsxNode::Text(text) if !text.trim().is_empty())
			|| matches!(node, BsxNode::Expr(_))
	};
	let keep: Vec<bool> = children
		.iter()
		.enumerate()
		.map(|(index, node)| {
			let insignificant = is_blank_text(node)
				&& index
					.checked_sub(1)
					.and_then(|prev| children.get(prev))
					.is_none_or(|prev| !is_inline(prev))
				&& children.get(index + 1).is_none_or(|next| !is_inline(next));
			!insignificant
		})
		.collect();
	let mut keep = keep.into_iter();
	children.retain(|_| keep.next().unwrap_or(true));
}

/// Wrap maximal runs of inline children in anonymous `<div>` blocks whenever the
/// children mix inline and block content, mirroring how a browser generates
/// anonymous block boxes. Without this a list item like `` - a: `b` `` followed
/// by a nested list lays its `a: b` out as two stacked full-width blocks rather
/// than one inline line. A purely-inline item is returned unchanged so it still
/// flows as a single inline context.
fn wrap_inline_runs(children: Vec<BsxNode>) -> Vec<BsxNode> {
	if !children.iter().any(is_block_level) {
		return children;
	}
	let mut out = Vec::new();
	let mut run: Vec<BsxNode> = Vec::new();
	let flush = |run: &mut Vec<BsxNode>, out: &mut Vec<BsxNode>| {
		// a run of only whitespace carries no content, so it's dropped rather than
		// boxed into an empty block.
		match run.iter().any(|node| !is_blank_text(node)) {
			true => {
				let span = match (run.first(), run.last()) {
					(Some(first), Some(last)) => first.span().to(last.span()),
					_ => default(),
				};
				out.push(element("div", vec![], core::mem::take(run), span));
			}
			false => run.clear(),
		}
	};
	for child in children {
		if is_block_level(&child) {
			flush(&mut run, &mut out);
			out.push(child);
		} else {
			run.push(child);
		}
	}
	flush(&mut run, &mut out);
	out
}

/// Internal stack-based builder that converts pulldown events into a
/// [`BsxNode`] tree. Each open tag pushes a frame; each close tag pops
/// one and appends the completed subtree to the parent.
struct MdTreeBuilder<'a> {
	/// The full input text, used for slicing source spans.
	text: &'a str,
	/// Stack of open elements.
	/// The bottom entry is the synthetic root.
	stack: Vec<StackFrame<'a>>,
	/// Collected frontmatter content, its format and where it starts.
	frontmatter_content: Option<(String, MediaType, Option<usize>)>,
	/// Whether we're inside a metadata block.
	in_metadata: bool,
	/// How embedded markup is read.
	fragment: BsxFragmentConfig,
	/// The formats a fence may be a document of.
	parsers: &'a DocParsers,
	/// The fence being read as a document, if one is.
	fence: Option<Fence>,
}

/// A fence read as a document rather than as code.
struct Fence {
	media_type: MediaType,
	info: FenceInfo,
	source: String,
	/// Where the source stands in the page, `None` once it is in pieces, ie
	/// within a quote.
	start: Option<usize>,
	/// Where the fence was written.
	span: SourceSpan,
}

impl Fence {
	/// Add `text`, the next of the fence's source, written at `range`.
	fn push(&mut self, text: &str, range: &Range<usize>) {
		let follows = match self.source.is_empty() {
			true => {
				self.start = Some(range.start);
				true
			}
			false => self
				.start
				.is_some_and(|start| start + self.source.len() == range.start),
		};
		// a piece that does not follow, or is not as written, has no one place
		if !follows || range.len() != text.len() {
			self.start = None;
		}
		self.source.push_str(text);
	}
}

struct StackFrame<'a> {
	/// HTML tag name for this element. Markdown tags are `'static` literals;
	/// embedded markup tags borrow from the source, hence the `'a` lifetime.
	name: &'a str,
	/// Attributes on this element, sharing the BSX value grammar.
	attributes: Vec<BsxAttribute>,
	/// Accumulated child nodes.
	children: Vec<BsxNode>,
	/// Where it was written, so far.
	span: SourceSpan,
	/// Whether embedded markup opened it rather than prose.
	embedded: bool,
}

impl Fence {
	/// The fence `info` opens, when its first word names a media type one of
	/// `parsers` reads. A language (`json`, `html`) names none, so a code
	/// sample stays code.
	fn of(
		info: &str,
		parsers: &DocParsers,
		span: impl Into<SourceSpan>,
	) -> Option<Self> {
		let (word, info) = match info.trim().split_once(char::is_whitespace) {
			Some((word, info)) => (word, info.trim()),
			None => (info.trim(), ""),
		};
		let media_type = MediaType::from_content_type(word);
		(!word.is_empty() && parsers.reads(&media_type)).then(|| Self {
			media_type,
			info: FenceInfo::new(info),
			source: default(),
			start: None,
			span: span.into(),
		})
	}
}

impl<'a> StackFrame<'a> {
	fn new(name: &'a str, span: impl Into<SourceSpan>) -> Self {
		Self::with_attributes(name, Vec::new(), span)
	}

	fn with_attributes(
		name: &'a str,
		attributes: Vec<BsxAttribute>,
		span: impl Into<SourceSpan>,
	) -> Self {
		Self {
			name,
			attributes,
			children: Vec::new(),
			span: span.into(),
			embedded: false,
		}
	}
}

impl<'a> MdTreeBuilder<'a> {
	fn new(
		text: &'a str,
		fragment: BsxFragmentConfig,
		parsers: &'a DocParsers,
	) -> Self {
		Self {
			text,
			// root frame collects top-level nodes
			stack: vec![StackFrame::new("", SourceSpan::default())],
			frontmatter_content: None,
			in_metadata: false,
			fragment,
			parsers,
			fence: None,
		}
	}

	/// Get a `&str` slice of the input for the given byte range.
	fn slice(&self, range: &Range<usize>) -> &'a str {
		&self.text[range.start..range.end.min(self.text.len())]
	}

	/// Where `slice`, a slice of the input, stands in it.
	fn span_of(&self, slice: &str) -> Range<usize> {
		let start = slice.as_ptr() as usize - self.text.as_ptr() as usize;
		start..start + slice.len()
	}

	/// Push a new element frame onto the stack.
	fn push(&mut self, frame: StackFrame<'a>) { self.stack.push(frame); }

	/// Pop the top frame, build an element, and append it
	/// to the parent frame.
	fn pop(&mut self) {
		if self.stack.len() <= 1 {
			return;
		}
		if let Some(mut frame) = self.stack.pop() {
			// drop the formatting indentation an embedded-markup element carries
			// between its child tags, the markdown twin of the BSX parser's
			// `normalize_whitespace`; skip `<pre>`-family frames whose whitespace is
			// significant.
			if !Element::PRE_ELEMENTS.contains(&frame.name) {
				drop_insignificant_whitespace(&mut frame.children);
			}
			// a tight list item mixing leading inline content with a nested list (or
			// other block) is the one prose case of a block box with mixed children;
			// wrap its inline runs in anonymous blocks so they flow on one line
			// rather than each breaking to its own (browsers do this implicitly).
			if frame.name == "li" {
				frame.children = wrap_inline_runs(frame.children);
			}
			let node = element(
				frame.name,
				frame.attributes,
				frame.children,
				frame.span,
			);
			if let Some(parent) = self.stack.last_mut() {
				// the implicit body reaches to its last row
				if parent.name == "tbody" {
					parent.span = parent.span.to(node.span());
				}
				parent.children.push(node);
			}
		}
	}

	/// Append a leaf node to the current frame.
	fn push_leaf(&mut self, node: BsxNode) {
		if let Some(parent) = self.stack.last_mut() {
			parent.children.push(node);
		}
	}

	/// Append a void element (no children) to the current frame.
	fn push_void(
		&mut self,
		name: &str,
		attributes: Vec<BsxAttribute>,
		span: impl Into<SourceSpan>,
	) {
		self.push_leaf(element(name, attributes, vec![], span));
	}

	fn handle_event(
		&mut self,
		event: Event<'a>,
		range: Range<usize>,
	) -> Result {
		let source = self.slice(&range);

		match event {
			Event::Start(tag) => self.handle_start(tag, source, range),
			Event::End(TagEnd::CodeBlock) if self.fence.is_some() => {
				self.finish_fence()?
			}
			Event::End(TagEnd::HtmlBlock) => {}
			Event::End(_) => self.handle_end(),
			Event::Text(text) => {
				if self.in_metadata {
					// accumulate metadata text
					if let Some((content, _, start)) =
						&mut self.frontmatter_content
					{
						start.get_or_insert(range.start);
						content.push_str(&text);
					}
				} else if let Some(fence) = &mut self.fence {
					fence.push(&text, &range);
				} else if self.fragment.expressions {
					self.push_text_with_expressions(self.slice(&range))?;
				} else {
					self.push_leaf(text_node(source, range));
				}
			}
			Event::Code(text) => {
				// Inline code: <code>text</code>
				let text_slice = self.slice(&range);
				// the source range includes the backticks, the inner text is what
				// pulldown gives us
				let inner = self.find_inner_text(text_slice, &text);
				let inner = text_node(inner, self.span_of(inner));
				self.push_leaf(element("code", vec![], vec![inner], range));
			}
			Event::SoftBreak => {
				self.push_leaf(text_node(source, range));
			}
			Event::HardBreak => {
				self.push_void("br", vec![], range);
			}
			Event::Rule => {
				self.push_void("hr", vec![], range);
			}
			// Both block and inline embedded HTML are tokenized and applied to
			// the builder stack, so they become real elements rather than
			// verbatim text. `source` is a slice of the original `'a` input, so
			// the parsed nodes borrow from it directly.
			Event::Html(_) | Event::InlineHtml(_) => {
				self.apply_html_fragment(source, range)?;
			}
			Event::FootnoteReference(label) => {
				// <sup><a href="#{label}">{label}</a></sup>, linking the
				// definition whose `id` is the label
				let label_text = self
					.find_substring(source, label.as_ref())
					.unwrap_or(source);
				let label = text_node(label_text, range.clone());
				let link = element(
					"a",
					vec![str_attr("href", &format!("#{label_text}"))],
					vec![label],
					range.clone(),
				);
				self.push_leaf(element("sup", vec![], vec![link], range));
			}
			Event::TaskListMarker(checked) => {
				let mut attrs =
					vec![flag_attr("disabled"), str_attr("type", "checkbox")];
				if checked {
					attrs.push(flag_attr("checked"));
				}
				self.push_void("input", attrs, range);
			}
			Event::InlineMath(_text) => {
				let text_slice = self.slice(&range);
				let text = text_node(text_slice, range.clone());
				self.push_leaf(element(
					"span",
					vec![str_attr("class", "math-inline")],
					vec![text],
					range,
				));
			}
			Event::DisplayMath(_text) => {
				let text_slice = self.slice(&range);
				let text = text_node(text_slice, range.clone());
				self.push_leaf(element(
					"div",
					vec![str_attr("class", "math-display")],
					vec![text],
					range,
				));
			}
		}
		Ok(())
	}

	fn handle_start(
		&mut self,
		tag: Tag<'a>,
		source: &'a str,
		range: Range<usize>,
	) {
		// an element's event spans the whole of it
		let span = SourceSpan::from(range.clone());
		match tag {
			Tag::Paragraph => {
				self.push(StackFrame::new("p", span));
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
				self.push(StackFrame::new(name, span));
			}
			Tag::BlockQuote(_) => {
				self.push(StackFrame::new("blockquote", span));
			}
			Tag::CodeBlock(CodeBlockKind::Fenced(ref info))
				if let Some(fence) =
					Fence::of(info, self.parsers, range.clone()) =>
			{
				self.fence = Some(fence);
			}
			Tag::CodeBlock(kind) => {
				// <pre><code class="{lang}"> — class is the bare language
				// name (borrowed from source) rather than "language-X",
				// since attributes must be zero-copy slices. An info string
				// carrying more than the language (```mermaid text) is kept
				// whole as `data-info`, so a consumer reads its extra words.
				self.push(StackFrame::new("pre", span.clone()));
				let attrs = match kind {
					CodeBlockKind::Fenced(info) => {
						let info_str = info.as_ref().trim();
						let mut words = info_str.split_whitespace();
						let lang = words.next().unwrap_or("");
						let mut attrs = Vec::new();
						if !lang.is_empty() {
							// Find the language string within source for borrowing
							let class_val = self
								.find_substring(source, lang)
								.unwrap_or(source);
							attrs.push(str_attr("class", class_val));
						}
						if words.next().is_some() {
							let info_val = self
								.find_substring(source, info_str)
								.unwrap_or(info_str);
							attrs.push(str_attr("data-info", info_val));
						}
						attrs
					}
					CodeBlockKind::Indented => vec![],
				};
				self.push(StackFrame::with_attributes("code", attrs, span));
			}
			// the block's markup drives the stack as it streams in, so an
			// element it opens stays open for the markdown that follows
			Tag::HtmlBlock => {}
			Tag::List(start) => match start {
				Some(_start_num) => {
					// Find start number text in source for attribute borrowing
					let start_text =
						self.find_digit_substring(source).unwrap_or(source);
					let attrs = vec![str_attr("start", start_text)];
					self.push(StackFrame::with_attributes("ol", attrs, span));
				}
				None => {
					self.push(StackFrame::new("ul", span));
				}
			},
			Tag::Item => {
				self.push(StackFrame::new("li", span));
			}
			Tag::FootnoteDefinition(label) => {
				let label_text = self
					.find_substring(source, label.as_ref())
					.unwrap_or(source);
				let attrs = vec![
					str_attr("class", "footnote-definition"),
					str_attr("id", label_text),
				];
				self.push(StackFrame::with_attributes("div", attrs, span));
			}
			Tag::DefinitionList => {
				self.push(StackFrame::new("dl", span));
			}
			Tag::DefinitionListTitle => {
				self.push(StackFrame::new("dt", span));
			}
			Tag::DefinitionListDefinition => {
				self.push(StackFrame::new("dd", span));
			}
			Tag::Table(_alignments) => {
				// every table sits in its scroll wrapper
				let attrs =
					vec![str_attr("class", html_ext::TABLE_SCROLL_CLASS)];
				self.push(StackFrame::with_attributes(
					"div",
					attrs,
					span.clone(),
				));
				self.push(StackFrame::new("table", span));
			}
			Tag::TableHead => {
				// the head's cells come without a row; a browser parsing the
				// served table gives them one, so the tree carries it too
				self.push(StackFrame::new("thead", span.clone()));
				self.push(StackFrame::new("tr", span));
			}
			Tag::TableRow => {
				// a body row lands in a `<tbody>` the parser never emits: a
				// browser parsing the served table inserts one, and a page
				// the world adopts must agree with it node for node
				if self.stack.last().is_some_and(|frame| frame.name == "table")
				{
					self.push(StackFrame::new("tbody", span.clone()));
				}
				self.push(StackFrame::new("tr", span));
			}
			Tag::TableCell => {
				// We use td for all cells; renderers can differentiate
				// based on whether the parent is thead or tbody
				self.push(StackFrame::new("td", span));
			}
			Tag::Emphasis => {
				self.push(StackFrame::new("em", span));
			}
			Tag::Strong => {
				self.push(StackFrame::new("strong", span));
			}
			Tag::Strikethrough => {
				self.push(StackFrame::new("del", span));
			}
			Tag::Superscript => {
				self.push(StackFrame::new("sup", span));
			}
			Tag::Subscript => {
				self.push(StackFrame::new("sub", span));
			}
			Tag::Link {
				dest_url, title, ..
			} => {
				let href = self
					.find_substring(source, dest_url.as_ref())
					.unwrap_or(source);
				let mut attrs = vec![str_attr("href", href)];
				if !title.is_empty() {
					let title_text = self
						.find_substring(source, title.as_ref())
						.unwrap_or(source);
					attrs.push(str_attr("title", title_text));
				}
				self.push(StackFrame::with_attributes("a", attrs, span));
			}
			Tag::Image {
				dest_url, title, ..
			} => {
				let src = self
					.find_substring(source, dest_url.as_ref())
					.unwrap_or(source);
				let mut attrs = vec![str_attr("src", src)];
				if !title.is_empty() {
					let title_text = self
						.find_substring(source, title.as_ref())
						.unwrap_or(source);
					attrs.push(str_attr("title", title_text));
				}
				self.push(StackFrame::with_attributes("img", attrs, span));
			}
			Tag::MetadataBlock(kind) => {
				let media_type = match kind {
					pulldown_cmark::MetadataBlockKind::YamlStyle => {
						MediaType::Yaml
					}
					pulldown_cmark::MetadataBlockKind::PlusesStyle => {
						MediaType::Toml
					}
				};
				self.frontmatter_content =
					Some((String::new(), media_type, None));
				self.in_metadata = true;
				// Push a dummy frame so pop works correctly
				self.push(StackFrame::new("__metadata", span));
			}
		}
	}

	fn handle_end(&mut self) {
		// a prose element closes what markup left open within it
		while self.stack.len() > 1
			&& self.stack.last().is_some_and(|frame| frame.embedded)
		{
			self.pop();
		}
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
			"img" => {
				// `img` is a void element. pulldown-cmark emits alt text
				// as child Text events; collect them into an `alt`
				// attribute and clear children so the element is void.
				if let Some(frame) = self.stack.last_mut() {
					// simple alt text is a single text node
					let alt =
						frame.children.iter().find_map(|node| match node {
							BsxNode::Text(text) => Some(text.value.clone()),
							_ => None,
						});
					if let Some(alt) = alt {
						frame.attributes.push(str_attr("alt", &alt));
					}
					frame.children.clear();
				}
				self.pop();
			}
			"code" => {
				// a fenced code block pushes `pre > code`; one `End(CodeBlock)`
				// closes both, else the trailing `pre` would swallow every
				// following block as a child.
				self.pop();
				self.pop();
			}
			"tbody" => {
				// the implicit body opened by the first body row closes with
				// its table: one `End(Table)` pops it, the table and its wrapper
				self.pop();
				self.pop_table();
			}
			"table" => {
				// a head-only table: `End(Table)` pops it and its wrapper
				self.pop_table();
			}
			"tr" if self.stack[self.stack.len() - 2].name == "thead" => {
				// the head's implicit row closes with the head
				self.pop();
				self.pop();
			}
			_ => {
				self.pop();
			}
		}
	}

	/// Read the fence just closed as the document it holds.
	fn finish_fence(&mut self) -> Result {
		let Some(fence) = self.fence.take() else {
			return Ok(());
		};
		let mut parsers = self.parsers.clone();
		parsers.edit_setting(|info: &mut FenceInfo| *info = fence.info);
		let bytes = MediaBytes::new_str(fence.media_type, &fence.source);
		let mut document = parsers.parse(&bytes).map_err(|error| {
			match (error.into_source(), fence.start) {
				// where in the page, when the source stands there whole
				(error, Some(start)) => SourceError::within(error, start),
				(error, None) => fence.span.locate(SourceError::split(error).1),
			}
		})?;
		match fence.start {
			Some(start) => document.spans.shift(start as u32),
			None => document.spans = default(),
		}
		self.push_leaf(BsxNode::Embedded(Spanned::new(document, fence.span)));
		Ok(())
	}

	/// Tokenize an embedded HTML fragment and apply its tokens to the builder
	/// stack, turning embedded HTML into real elements rather than verbatim
	/// text.
	///
	/// Open tags push a frame, close tags pop the matching one, and
	/// void/self-closing tags become leaf elements. Driving the shared stack
	/// (rather than building a sub-tree) lets a single element span several
	/// `Event::Html` lines — the open tag on one line, the close on another —
	/// and lets a paired inline tag (`<strong>…</strong>`) wrap the markdown
	/// text events that arrive between its open and close.
	fn apply_html_fragment(
		&mut self,
		source: &'a str,
		range: Range<usize>,
	) -> Result {
		let tokens = match BsxFragmentToken::parse_fragment(
			self.text,
			range.start..range.end.min(self.text.len()),
			&self.fragment,
		) {
			Ok(tokens) => tokens,
			// an unparseable fragment falls back to verbatim text
			Err(_) => {
				self.push_leaf(text_node(source, range));
				return Ok(());
			}
		};
		for Spanned { value: token, span } in tokens {
			match token {
				BsxFragmentToken::Open {
					tag,
					attributes,
					self_closing,
				} => {
					if self_closing || is_void_element(tag) {
						self.push_void(tag, attributes, span);
					} else {
						let mut frame =
							StackFrame::with_attributes(tag, attributes, span);
						frame.embedded = true;
						self.push(frame);
					}
				}
				BsxFragmentToken::Close { tag } => self.pop_named(tag, &span),
				BsxFragmentToken::Text(text) => {
					self.push_leaf(text_node(text, span))
				}
				BsxFragmentToken::Comment(text) => self.push_leaf(
					BsxNode::Comment(Spanned::new(text.to_string(), span)),
				),
				BsxFragmentToken::Doctype(text) => self.push_leaf(
					BsxNode::Doctype(Spanned::new(text.to_string(), span)),
				),
				BsxFragmentToken::Expr(expr) => self.push_expr(expr)?,
			}
		}
		Ok(())
	}

	/// Append an expression leaf, ie `@doc:count`.
	fn push_expr(&mut self, expr: &'a str) -> Result {
		let range = self.span_of(expr);
		let value = ValueExpr::parse_within(self.text, range.clone()).map_err(
			|error| {
				// named whole, at the place within it
				let (at, error) = SourceError::split(error);
				SourceError::at(
					bevyhow!("in `{{{expr}}}`: {error}"),
					at.map_or(range.start, |at| at as usize),
				)
			},
		)?;
		self.push_leaf(BsxNode::Expr(Spanned::new(value, range)));
		Ok(())
	}

	/// Pop a `table` frame and the scroll wrapper `Tag::Table` pushed under it.
	fn pop_table(&mut self) {
		self.pop();
		self.pop();
	}

	/// Pop the top frame only when its tag matches `name`, building its element
	/// and appending it to the parent. A mismatched close tag is ignored so a
	/// stray `</p>` cannot unwind the surrounding markdown structure.
	fn pop_named(&mut self, name: &str, close: &SourceSpan) {
		let matches = self.stack.len() > 1
			&& self
				.stack
				.last()
				.is_some_and(|frame| frame.name.eq_ignore_ascii_case(name));
		if matches {
			if let Some(frame) = self.stack.last_mut() {
				frame.span = frame.span.to(close);
			}
			self.pop();
		}
	}

	/// Try to find a substring within the source slice, returning a
	/// borrowed slice from the original text.
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
	/// [`BsxNode::Text`] and [`BsxNode::Expr`] leaves.
	///
	/// Supports nested braces, ie `{a {b} c}` is a single expression.
	fn push_text_with_expressions(&mut self, source: &'a str) -> Result {
		let mut remaining = source;
		while !remaining.is_empty() {
			if let Some(brace_pos) = remaining.find('{') {
				// push any text before the brace
				if brace_pos > 0 {
					let lead = &remaining[..brace_pos];
					self.push_leaf(text_node(lead, self.span_of(lead)));
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
					self.push_expr(expr)?;
					remaining = &after_brace[end_idx + 1..];
				} else {
					// unmatched brace, treat the rest as text
					self.push_leaf(text_node(
						remaining,
						self.span_of(remaining),
					));
					break;
				}
			} else {
				self.push_leaf(text_node(remaining, self.span_of(remaining)));
				break;
			}
		}
		Ok(())
	}

	/// Consume the builder and produce the final result.
	fn finish(mut self) -> Result<MarkdownTree> {
		// Drain any unclosed elements
		while self.stack.len() > 1 {
			self.pop();
		}

		let mut nodes = self
			.stack
			.pop()
			.map(|frame| frame.children)
			.unwrap_or_default();
		// the synthetic root collects top-level nodes; drop the whitespace between
		// top-level embedded-markup blocks too (eg a leading newline before
		// `<article>` that would otherwise paint as a blank first row).
		drop_insignificant_whitespace(&mut nodes);

		// a block its format refuses refuses the document, rather than
		// leaving it without the metadata it declares
		let frontmatter = self
			.frontmatter_content
			.map(|(content, media_type, start)| {
				Frontmatter::parse_at(
					&content,
					media_type,
					start.unwrap_or_default(),
				)
			})
			.transpose()?;

		Ok(MarkdownTree { nodes, frontmatter })
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn build(text: &str) -> Vec<BsxNode> {
		MarkdownTree::build(text, &MarkdownParser::new(), &default())
			.unwrap()
			.nodes
	}

	fn node_name(node: &BsxNode) -> &str {
		match node {
			BsxNode::Element(element) => &element.tag,
			BsxNode::Text(text)
			| BsxNode::Comment(text)
			| BsxNode::Doctype(text) => text,
			_ => "?",
		}
	}

	fn node_children(node: &BsxNode) -> &[BsxNode] {
		match node {
			BsxNode::Element(element) => &element.children,
			_ => &[],
		}
	}

	/// A component tag keeps its children where the HTML void element sharing
	/// its letters has none: `<Link>` once matched the void `link`, and its
	/// label leaked out as a sibling.
	#[beet_core::test]
	fn a_component_tag_is_not_void() {
		is_void_element("link").xpect_true();
		let nodes = build("<Link href=\"/x\">hello</Link>");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("Link");
		node_children(&nodes[0]).len().xpect_eq(1);
	}

	#[beet_core::test]
	fn paragraph() {
		let nodes = build("Hello world");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("p");
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(1);
		matches!(&children[0], BsxNode::Text(_)).xpect_true();
	}

	#[beet_core::test]
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

	#[beet_core::test]
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

	#[beet_core::test]
	fn unordered_list() {
		let nodes = build("- a\n- b");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("ul");
		let items = node_children(&nodes[0]);
		items.len().xpect_eq(2);
		node_name(&items[0]).xpect_eq("li");
		node_name(&items[1]).xpect_eq("li");
	}

	#[beet_core::test]
	fn ordered_list() {
		let nodes = build("1. a\n2. b");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("ol");
	}

	#[beet_core::test]
	fn code_block() {
		let nodes = build("```rust\nfn main() {}\n```");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("pre");
		let pre_children = node_children(&nodes[0]);
		pre_children.len().xpect_eq(1);
		node_name(&pre_children[0]).xpect_eq("code");
	}

	#[beet_core::test]
	fn code_block_info_string() {
		let attrs_of = |md: &str| {
			let nodes = build(md);
			match &node_children(&nodes[0])[0] {
				BsxNode::Element(element) => element
					.attributes
					.iter()
					.map(|attr| {
						(attr.key.clone(), match &attr.value {
							AttrValue::Str(value) => value.clone(),
							other => format!("{other:?}"),
						})
					})
					.collect::<Vec<_>>(),
				_ => panic!("expected the code element"),
			}
		};
		// a bare language is the class alone
		attrs_of("```rust\nfn main() {}\n```")
			.xpect_eq(vec![("class".to_string(), "rust".to_string())]);
		// extra info words ride along whole as `data-info`
		attrs_of("```mermaid text\ngraph LR; A-->B\n```").xpect_eq(vec![
			("class".to_string(), "mermaid".to_string()),
			("data-info".to_string(), "mermaid text".to_string()),
		]);
	}

	#[beet_core::test]
	fn code_block_closes_pre() {
		// the paragraph after a fenced block is a sibling of the `pre`, not a
		// child swallowed by an unclosed `pre`.
		let nodes = build("```rust\nfn main() {}\n```\n\nafter");
		nodes.len().xpect_eq(2);
		node_name(&nodes[0]).xpect_eq("pre");
		node_name(&nodes[1]).xpect_eq("p");
	}

	#[beet_core::test]
	fn inline_code() {
		let nodes = build("use `foo()` here");
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(|child| node_name(child) == "code")
			.xpect_true();
	}

	#[beet_core::test]
	fn thematic_break() {
		let nodes = build("---");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("hr");
	}

	#[beet_core::test]
	fn link() {
		let nodes = build("[click](https://example.com)");
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(|child| node_name(child) == "a")
			.xpect_true();
	}

	#[beet_core::test]
	fn image() {
		let nodes = build("![alt](img.png)");
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(|child| node_name(child) == "img")
			.xpect_true();
	}

	#[beet_core::test]
	fn blockquote() {
		let nodes = build("> quoted");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("blockquote");
	}

	/// A footnote reference links its definition by the `id` the
	/// definition carries, its label.
	#[beet_core::test]
	fn footnote_links_its_definition() {
		let nodes = build("words[^note]\n\n[^note]: the note");
		let reference = &node_children(&nodes[0])[1];
		node_name(reference).xpect_eq("sup");
		let BsxNode::Element(link) = &node_children(reference)[0] else {
			panic!("the reference holds its link");
		};
		link.attributes[0]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("#note".into()));
		let BsxNode::Element(definition) = &nodes[1] else {
			panic!("the definition is an element");
		};
		definition.attributes[1]
			.value
			.clone()
			.xpect_eq(AttrValue::Str("note".into()));
	}

	#[beet_core::test]
	fn strikethrough() {
		let nodes = build("~~deleted~~");
		let children = node_children(&nodes[0]);
		children
			.iter()
			.any(|child| node_name(child) == "del")
			.xpect_true();
	}

	/// A table is `thead` then `tbody`, the shape a browser parses a served
	/// table into, so the world's tree and the parsed DOM agree, in the scroll
	/// wrapper every table sits in.
	#[beet_core::test]
	fn table() {
		let nodes =
			build("| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n\nafter");
		nodes.len().xpect_eq(2);
		let table = table_in_wrapper(&nodes[0]);
		let sections = node_children(table);
		sections
			.iter()
			.map(node_name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["thead", "tbody"]);
		let head_rows = node_children(&sections[0]);
		head_rows.len().xpect_eq(1);
		node_name(&head_rows[0]).xpect_eq("tr");
		node_children(&head_rows[0]).len().xpect_eq(2);
		node_children(&sections[1]).len().xpect_eq(2);
		node_name(&nodes[1]).xpect_eq("p");
	}

	/// A head-only table closes its wrapper too, so the block after it is a
	/// sibling rather than a child of the wrapper.
	#[beet_core::test]
	fn head_only_table() {
		let nodes = build("| A | B |\n|---|---|\n\nafter");
		nodes.len().xpect_eq(2);
		let table = table_in_wrapper(&nodes[0]);
		node_children(table)
			.iter()
			.map(node_name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["thead"]);
		node_name(&nodes[1]).xpect_eq("p");
	}

	/// The `<table>` inside a `div.table-scroll` wrapper node.
	fn table_in_wrapper(wrapper: &BsxNode) -> &BsxNode {
		node_name(wrapper).xpect_eq("div");
		let table = &node_children(wrapper)[0];
		node_name(table).xpect_eq("table");
		table
	}

	#[beet_core::test]
	fn multiple_blocks() {
		let nodes = build("# Title\n\nParagraph");
		nodes.len().xpect_eq(2);
		node_name(&nodes[0]).xpect_eq("h1");
		node_name(&nodes[1]).xpect_eq("p");
	}

	#[beet_core::test]
	fn nested_list() {
		let nodes = build("- a\n  - b\n  - c\n- d");
		nodes.len().xpect_eq(1);
		node_name(&nodes[0]).xpect_eq("ul");
		let items = node_children(&nodes[0]);
		items.len().xpect_eq(2);
	}

	#[beet_core::test]
	fn embedded_block_html_becomes_element() {
		// a block-level HTML element is parsed into a real element, not
		// verbatim text, so its tag drives layout/styling (eg `<iframe>` is
		// non-visual and `<br>` is a line break).
		let nodes =
			build("text\n\n<iframe src=\"https://x\"></iframe>\n\ntext");
		nodes
			.iter()
			.any(|node| node_name(node) == "iframe")
			.xpect_true();
	}

	#[beet_core::test]
	fn embedded_void_block_html_becomes_element() {
		let nodes = build("a\n\n<br/>\n<br/>\n\nb");
		nodes
			.iter()
			.filter(|node| node_name(node) == "br")
			.count()
			.xpect_eq(2);
	}

	/// An element a block of markup opens holds the prose after it, up to the
	/// block that closes it.
	#[beet_core::test]
	fn embedded_markup_wraps_prose() {
		let nodes = build("<div>\n\n*a*\n\n- b\n\n</div>\n\nc");
		nodes
			.iter()
			.map(node_name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["div", "p"]);
		node_children(&nodes[0])
			.iter()
			.map(node_name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["p", "ul"]);
	}

	/// Markup left open in a prose element closes with it.
	#[beet_core::test]
	fn unclosed_markup_closes_with_its_prose() {
		let nodes = build("- <b>a\n- c\n\nd");
		nodes
			.iter()
			.map(node_name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["ul", "p"]);
		node_children(&nodes[0]).len().xpect_eq(2);
	}

	#[beet_core::test]
	fn embedded_markup_drops_inter_tag_whitespace() {
		// the formatting indentation between embedded-markup child tags is dropped
		// (the markdown twin of the BSX parser's `normalize_whitespace`), so a design
		// row renders its items with no blank rows / extra gap on the charcell
		// target — the parity bug the `.mdx` design showcases surfaced.
		let nodes = build(
			"<div class=\"row\">\n\t<span>a</span>\n\t<span>b</span>\n</div>",
		);
		let div = nodes.iter().find(|node| node_name(node) == "div").unwrap();
		// only the two spans survive; the `\n\t` text runs between/around them drop.
		let names: Vec<&str> =
			node_children(div).iter().map(node_name).collect();
		names.xpect_eq(vec!["span", "span"]);
	}

	#[beet_core::test]
	fn embedded_markup_keeps_inline_whitespace() {
		// whitespace between an inline element and real text is significant (a word
		// gap), so it must survive even as the inter-tag indentation drops.
		let nodes = build("<p>see <a href=\"#\">here</a> now</p>");
		let para = nodes.iter().find(|node| node_name(node) == "p").unwrap();
		let texts: Vec<&str> = node_children(para)
			.iter()
			.filter_map(|node| match node {
				BsxNode::Text(text) => Some(text.as_str()),
				_ => None,
			})
			.collect();
		// the leading "see " and trailing " now" keep their boundary spaces.
		texts.iter().any(|text| text.ends_with(' ')).xpect_true();
		texts.iter().any(|text| text.starts_with(' ')).xpect_true();
	}

	#[beet_core::test]
	fn inline_html_wraps_markdown_text() {
		// a paired inline tag wraps the markdown text that streams between its
		// open and close events, becoming a real element around the text.
		let nodes = build("Hello <strong>brave</strong> world");
		let para = &nodes[0];
		node_name(para).xpect_eq("p");
		let strong = node_children(para)
			.iter()
			.find(|node| node_name(node) == "strong")
			.unwrap();
		matches!(&node_children(strong)[0], BsxNode::Text(text) if text.value == "brave")
			.xpect_true();
	}

	#[beet_core::test]
	fn frontmatter_parsed() {
		let result = MarkdownTree::build(
			"---\ntitle: Hello\n---\n\n# Title",
			&MarkdownParser::new(),
			&default(),
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

	#[beet_core::test]
	fn no_frontmatter() {
		let result = MarkdownTree::build(
			"# Just a heading",
			&MarkdownParser::new(),
			&default(),
		)
		.unwrap();
		result.frontmatter.is_none().xpect_true();
	}

	/// A block its format refuses refuses the document with the reason,
	/// rather than leaving it without its metadata.
	#[beet_core::test]
	fn refuses_a_malformed_frontmatter() {
		MarkdownTree::build(
			"---\nlogo: &shared x\n---\n\n# Title",
			&MarkdownParser::new(),
			&default(),
		)
		.map(|_| ())
		.unwrap_err()
		.to_string()
		.xpect_contains("an anchor or alias");
	}

	fn build_mdx(text: &str) -> Vec<BsxNode> {
		MarkdownTree::build(text, &MarkdownParser::mdx(), &default())
			.unwrap()
			.nodes
	}

	/// The field `node` binds.
	fn bound(node: &BsxNode) -> String {
		match node {
			BsxNode::Expr(expr) => match &expr.value {
				ValueExpr::Binding(binding) => binding.field_path.to_string(),
				_ => default(),
			},
			_ => default(),
		}
	}

	#[beet_core::test]
	fn expression_in_paragraph() {
		let nodes = build_mdx("hello {@doc:name} world");
		nodes.len().xpect_eq(1);
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(3);
		node_name(&children[0]).xpect_eq("hello ");
		bound(&children[1]).xpect_eq("name");
		node_name(&children[2]).xpect_eq(" world");
	}

	#[beet_core::test]
	fn expression_only() {
		let nodes = build_mdx("{@doc:greeting}");
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(1);
		bound(&children[0]).xpect_eq("greeting");
	}

	#[beet_core::test]
	fn expression_nested_braces() {
		let nodes = build_mdx("{{a: {b: 1}}}");
		matches!(
			&node_children(&nodes[0])[0],
			BsxNode::Expr(Spanned {
				value: ValueExpr::Literal(DataLiteral::Struct(_)),
				..
			})
		)
		.xpect_true();
	}

	/// An expression that is none refuses the document, naming itself.
	#[beet_core::test]
	fn a_malformed_expression_is_refused() {
		MarkdownTree::build("{@nope:x}", &MarkdownParser::mdx(), &default())
			.map(|_| ())
			.unwrap_err()
			.to_string()
			.xpect_contains("@nope:x");
	}

	#[beet_core::test]
	fn no_expressions_in_markdown() {
		// braces are plain text in markdown proper
		let nodes = build("hello {name} world");
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(1);
		matches!(&children[0], BsxNode::Text(_)).xpect_true();
	}

	#[beet_core::test]
	fn multiple_expressions() {
		let nodes = build_mdx("{@doc:a} and {@doc:b}");
		let children = node_children(&nodes[0]);
		children.len().xpect_eq(3);
		bound(&children[0]).xpect_eq("a");
		node_name(&children[1]).xpect_eq(" and ");
		bound(&children[2]).xpect_eq("b");
	}
}
