//! Render content trees to HTML via the [`CardVisitor`] pattern.
//!
//! [`HtmlRenderer`] walks a card's entity tree and produces an HTML
//! string. It handles all block-level and inline elements, including
//! nested block quotes, lists, tables, and inline formatting markers
//! via [`InlineStyle`].
//!
//! The render tool that wires this up for card rendering lives in
//! the sibling [`html_render_tool`](super::html_render_tool) module.
//!
//! # Usage
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! fn render(walker: CardWalker, entity: Entity) -> String {
//!     let mut renderer = HtmlRenderer::new();
//!     walker.walk_card(&mut renderer, entity);
//!     renderer.finish()
//! }
//! ```
use crate::prelude::*;
use beet_core::prelude::*;
use std::ops::ControlFlow;

/// Visitor-based HTML renderer.
///
/// Accumulates HTML text as the [`CardWalker`] traverses the content
/// tree. Call [`finish`](Self::finish) after walking to retrieve the
/// final HTML string.
pub struct HtmlRenderer {
	/// Accumulated output.
	buffer: String,
	/// Stack of open tags for validation/debugging.
	tag_stack: Vec<&'static str>,
	/// Whether we are inside a table head section.
	in_table_head: bool,
	/// Whether we are inside a code block (skip inline formatting).
	in_code_block: bool,
}

impl HtmlRenderer {
	/// Create a new renderer with default settings.
	pub fn new() -> Self {
		Self {
			buffer: String::new(),
			tag_stack: Vec::new(),
			in_table_head: false,
			in_code_block: false,
		}
	}

	/// Consume the renderer and return the accumulated HTML.
	pub fn finish(self) -> String { self.buffer }

	/// Push an opening tag onto the buffer and tag stack.
	fn open_tag(&mut self, tag: &'static str) {
		self.buffer.push('<');
		self.buffer.push_str(tag);
		self.buffer.push('>');
		self.tag_stack.push(tag);
	}

	/// Push an opening tag with attributes onto the buffer.
	fn open_tag_with_attrs(&mut self, tag: &'static str, attrs: &str) {
		self.buffer.push('<');
		self.buffer.push_str(tag);
		self.buffer.push(' ');
		self.buffer.push_str(attrs);
		self.buffer.push('>');
		self.tag_stack.push(tag);
	}

	/// Push a closing tag onto the buffer and pop the tag stack.
	fn close_tag(&mut self, tag: &'static str) {
		self.buffer.push_str("</");
		self.buffer.push_str(tag);
		self.buffer.push('>');
		self.tag_stack.pop();
	}

	/// Push a self-closing tag onto the buffer.
	fn self_closing_tag(&mut self, tag: &str) {
		self.buffer.push('<');
		self.buffer.push_str(tag);
		self.buffer.push_str(" />");
	}

	/// Push a self-closing tag with attributes onto the buffer.
	fn self_closing_tag_with_attrs(&mut self, tag: &str, attrs: &str) {
		self.buffer.push('<');
		self.buffer.push_str(tag);
		self.buffer.push(' ');
		self.buffer.push_str(attrs);
		self.buffer.push_str(" />");
	}

	/// Escape text for safe HTML output.
	fn escape_html(text: &str) -> String {
		let mut escaped = String::with_capacity(text.len());
		for ch in text.chars() {
			match ch {
				'&' => escaped.push_str("&amp;"),
				'<' => escaped.push_str("&lt;"),
				'>' => escaped.push_str("&gt;"),
				'"' => escaped.push_str("&quot;"),
				'\'' => escaped.push_str("&#x27;"),
				_ => escaped.push(ch),
			}
		}
		escaped
	}

	/// Escape an attribute value.
	fn escape_attr(text: &str) -> String { Self::escape_html(text) }

	/// Push text content, applying inline styles from the context.
	fn push_styled_text(&mut self, text: &str, style: &InlineStyle) {
		// Open inline tags in a consistent order
		if style.contains(InlineStyle::QUOTE) {
			self.buffer.push_str("<q>");
		}
		if style.contains(InlineStyle::BOLD) {
			self.buffer.push_str("<strong>");
		}
		if style.contains(InlineStyle::ITALIC) {
			self.buffer.push_str("<em>");
		}
		if style.contains(InlineStyle::STRIKETHROUGH) {
			self.buffer.push_str("<del>");
		}
		if style.contains(InlineStyle::SUPERSCRIPT) {
			self.buffer.push_str("<sup>");
		}
		if style.contains(InlineStyle::SUBSCRIPT) {
			self.buffer.push_str("<sub>");
		}
		if style.contains(InlineStyle::CODE) {
			self.buffer.push_str("<code>");
		}
		if style.contains(InlineStyle::MATH_INLINE) {
			self.buffer.push_str("<span class=\"math-inline\">");
		}

		self.buffer.push_str(&Self::escape_html(text));

		// Close inline tags in reverse order
		if style.contains(InlineStyle::MATH_INLINE) {
			self.buffer.push_str("</span>");
		}
		if style.contains(InlineStyle::CODE) {
			self.buffer.push_str("</code>");
		}
		if style.contains(InlineStyle::SUBSCRIPT) {
			self.buffer.push_str("</sub>");
		}
		if style.contains(InlineStyle::SUPERSCRIPT) {
			self.buffer.push_str("</sup>");
		}
		if style.contains(InlineStyle::STRIKETHROUGH) {
			self.buffer.push_str("</del>");
		}
		if style.contains(InlineStyle::ITALIC) {
			self.buffer.push_str("</em>");
		}
		if style.contains(InlineStyle::BOLD) {
			self.buffer.push_str("</strong>");
		}
		if style.contains(InlineStyle::QUOTE) {
			self.buffer.push_str("</q>");
		}
	}
}

impl Default for HtmlRenderer {
	fn default() -> Self { Self::new() }
}

impl CardVisitor for HtmlRenderer {
	// -- Block-level visit --

	fn visit_heading(
		&mut self,
		_cx: &VisitContext,
		heading: &Heading,
	) -> ControlFlow<()> {
		let level = heading.level().min(6);
		let tag = match level {
			1 => "h1",
			2 => "h2",
			3 => "h3",
			4 => "h4",
			5 => "h5",
			_ => "h6",
		};
		self.open_tag(tag);
		ControlFlow::Continue(())
	}

	fn visit_paragraph(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.open_tag("p");
		ControlFlow::Continue(())
	}

	fn visit_block_quote(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.open_tag("blockquote");
		ControlFlow::Continue(())
	}

	fn visit_code_block(
		&mut self,
		_cx: &VisitContext,
		code_block: &CodeBlock,
	) -> ControlFlow<()> {
		self.in_code_block = true;
		self.buffer.push_str("<pre>");
		if let Some(lang) = &code_block.language {
			let escaped = Self::escape_attr(lang);
			self.buffer
				.push_str(&format!("<code class=\"language-{escaped}\">"));
		} else {
			self.buffer.push_str("<code>");
		}
		self.tag_stack.push("pre");
		ControlFlow::Continue(())
	}

	fn visit_list(
		&mut self,
		_cx: &VisitContext,
		list_marker: &ListMarker,
	) -> ControlFlow<()> {
		if list_marker.ordered {
			let start = list_marker.start.unwrap_or(1);
			if start != 1 {
				self.open_tag_with_attrs("ol", &format!("start=\"{start}\""));
			} else {
				self.open_tag("ol");
			}
		} else {
			self.open_tag("ul");
		}
		ControlFlow::Continue(())
	}

	fn visit_list_item(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.open_tag("li");
		ControlFlow::Continue(())
	}

	fn visit_table(
		&mut self,
		_cx: &VisitContext,
		_table: &Table,
	) -> ControlFlow<()> {
		self.open_tag("table");
		ControlFlow::Continue(())
	}

	fn visit_table_head(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.in_table_head = true;
		self.open_tag("thead");
		self.open_tag("tr");
		ControlFlow::Continue(())
	}

	fn visit_table_row(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.open_tag("tr");
		ControlFlow::Continue(())
	}

	fn visit_table_cell(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		if self.in_table_head {
			self.open_tag("th");
		} else {
			self.open_tag("td");
		}
		ControlFlow::Continue(())
	}

	fn visit_thematic_break(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.self_closing_tag("hr");
		ControlFlow::Continue(())
	}

	fn visit_image(
		&mut self,
		_cx: &VisitContext,
		image: &Image,
	) -> ControlFlow<()> {
		// We'll collect alt text from children, but for now open
		// a figure-like container. The actual <img> is emitted in
		// leave_image with the alt text collected from children.
		// For simplicity, emit the image tag immediately - alt text
		// will be handled by collecting text children.
		let src = Self::escape_attr(&image.src);
		let mut attrs = format!("src=\"{src}\"");
		if let Some(title) = &image.title {
			let escaped_title = Self::escape_attr(title);
			attrs.push_str(&format!(" title=\"{escaped_title}\""));
		}
		// alt text comes from child TextNode, but since we emit a
		// self-closing tag here, we need to handle this differently.
		// We'll use a simple approach: emit alt="" and let children
		// be ignored for images (they become alt text in markdown,
		// but in HTML the alt attribute is what matters).
		attrs.push_str(" alt=\"\"");
		self.self_closing_tag_with_attrs("img", &attrs);
		// Skip children since we've already emitted the tag
		ControlFlow::Break(())
	}

	fn visit_footnote_definition(
		&mut self,
		_cx: &VisitContext,
		footnote_def: &FootnoteDefinition,
	) -> ControlFlow<()> {
		let label = Self::escape_attr(&footnote_def.label);
		self.open_tag_with_attrs(
			"div",
			&format!("class=\"footnote\" id=\"fn-{label}\""),
		);
		self.buffer.push_str(&format!(
			"<sup><a href=\"#fnref-{label}\">{label}</a></sup> "
		));
		ControlFlow::Continue(())
	}

	fn visit_math_display(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.open_tag_with_attrs("div", "class=\"math-display\"");
		ControlFlow::Continue(())
	}

	fn visit_html_block(
		&mut self,
		_cx: &VisitContext,
		html_block: &HtmlBlock,
	) -> ControlFlow<()> {
		// Pass through raw HTML
		if !html_block.0.is_empty() {
			self.buffer.push_str(&html_block.0);
		}
		ControlFlow::Continue(())
	}

	fn visit_button(
		&mut self,
		_cx: &VisitContext,
		_button: &Button,
	) -> ControlFlow<()> {
		self.open_tag("button");
		ControlFlow::Continue(())
	}

	// -- Inline --

	fn visit_text(
		&mut self,
		cx: &VisitContext,
		text: &TextNode,
	) -> ControlFlow<()> {
		if self.in_code_block {
			self.buffer.push_str(&Self::escape_html(text.as_str()));
		} else {
			let style = cx.effective_style();
			self.push_styled_text(text.as_str(), &style);
		}
		ControlFlow::Continue(())
	}

	fn visit_link(
		&mut self,
		_cx: &VisitContext,
		link: &Link,
	) -> ControlFlow<()> {
		let href = Self::escape_attr(&link.href);
		let mut attrs = format!("href=\"{href}\"");
		if let Some(title) = &link.title {
			let escaped_title = Self::escape_attr(title);
			attrs.push_str(&format!(" title=\"{escaped_title}\""));
		}
		self.open_tag_with_attrs("a", &attrs);
		ControlFlow::Continue(())
	}

	fn visit_hard_break(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.self_closing_tag("br");
		ControlFlow::Continue(())
	}

	fn visit_soft_break(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.buffer.push('\n');
		ControlFlow::Continue(())
	}

	fn visit_footnote_ref(
		&mut self,
		_cx: &VisitContext,
		footnote_ref: &FootnoteRef,
	) -> ControlFlow<()> {
		let label = Self::escape_attr(&footnote_ref.label);
		self.buffer.push_str(&format!(
			"<sup id=\"fnref-{label}\"><a href=\"#fn-{label}\">{label}</a></sup>"
		));
		ControlFlow::Continue(())
	}

	fn visit_html_inline(
		&mut self,
		_cx: &VisitContext,
		html_inline: &HtmlInline,
	) -> ControlFlow<()> {
		// Pass through raw inline HTML
		self.buffer.push_str(&html_inline.0);
		ControlFlow::Continue(())
	}

	fn visit_task_list_check(
		&mut self,
		_cx: &VisitContext,
		task_check: &TaskListCheck,
	) -> ControlFlow<()> {
		if task_check.checked {
			self.self_closing_tag_with_attrs(
				"input",
				"type=\"checkbox\" checked disabled",
			);
		} else {
			self.self_closing_tag_with_attrs(
				"input",
				"type=\"checkbox\" disabled",
			);
		}
		self.buffer.push(' ');
		ControlFlow::Continue(())
	}

	// -- Block-level leave --

	fn leave_heading(&mut self, _cx: &VisitContext, heading: &Heading) {
		let tag = match heading.level().min(6) {
			1 => "h1",
			2 => "h2",
			3 => "h3",
			4 => "h4",
			5 => "h5",
			_ => "h6",
		};
		self.close_tag(tag);
	}

	fn leave_paragraph(&mut self, _cx: &VisitContext) { self.close_tag("p"); }

	fn leave_block_quote(&mut self, _cx: &VisitContext) {
		self.close_tag("blockquote");
	}

	fn leave_code_block(
		&mut self,
		_cx: &VisitContext,
		_code_block: &CodeBlock,
	) {
		self.in_code_block = false;
		self.buffer.push_str("</code></pre>");
		self.tag_stack.pop();
	}

	fn leave_list(&mut self, _cx: &VisitContext, list_marker: &ListMarker) {
		if list_marker.ordered {
			self.close_tag("ol");
		} else {
			self.close_tag("ul");
		}
	}

	fn leave_list_item(&mut self, _cx: &VisitContext) { self.close_tag("li"); }

	fn leave_table(&mut self, _cx: &VisitContext, _table: &Table) {
		self.close_tag("table");
	}

	fn leave_table_head(&mut self, _cx: &VisitContext) {
		self.close_tag("tr");
		self.close_tag("thead");
		self.in_table_head = false;
	}

	fn leave_table_row(&mut self, _cx: &VisitContext) { self.close_tag("tr"); }

	fn leave_table_cell(&mut self, _cx: &VisitContext) {
		if self.in_table_head {
			self.close_tag("th");
		} else {
			self.close_tag("td");
		}
	}

	fn leave_link(&mut self, _cx: &VisitContext, _link: &Link) {
		self.close_tag("a");
	}

	fn leave_image(&mut self, _cx: &VisitContext, _image: &Image) {
		// Image is self-closing, nothing to close
	}

	fn leave_html_block(
		&mut self,
		_cx: &VisitContext,
		_html_block: &HtmlBlock,
	) {
		// Raw HTML blocks have no wrapping tag to close
	}

	fn leave_button(&mut self, _cx: &VisitContext, _button: &Button) {
		self.close_tag("button");
	}
}


#[cfg(test)]
mod test {
	use super::*;

	/// Helper: spawn a card, walk it with `HtmlRenderer`, return
	/// the HTML string.
	fn render_card(world: &mut World, entity: Entity) -> String {
		world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut renderer = HtmlRenderer::new();
					walker.walk_card(&mut renderer, entity);
					renderer.finish()
				},
				entity,
			)
			.unwrap()
	}

	// -- Plain text --

	#[test]
	fn plain_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![TextNode::new("hello world")]))
			.id();
		render_card(&mut world, entity).xpect_eq("hello world");
	}

	#[test]
	fn multiple_segments() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				TextNode::new("hello"),
				TextNode::new(" "),
				TextNode::new("world"),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq("hello world");
	}

	// -- Inline formatting (container pattern) --

	#[test]
	fn important_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				TextNode::new("hello "),
				(Important, children![TextNode::new("bold")]),
				TextNode::new(" text"),
			]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("hello <strong>bold</strong> text");
	}

	#[test]
	fn emphasized_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				TextNode::new("hello "),
				(Emphasize, children![TextNode::new("italic")]),
				TextNode::new(" text"),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq("hello <em>italic</em> text");
	}

	#[test]
	fn code_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				TextNode::new("use "),
				(Code, children![TextNode::new("println!")]),
				TextNode::new(" macro"),
			]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("use <code>println!</code> macro");
	}

	#[test]
	fn quoted_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				TextNode::new("he said "),
				(Quote, children![TextNode::new("hello")]),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq("he said <q>hello</q>");
	}

	#[test]
	fn combined_markers() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Important, children![(
				Emphasize,
				children![TextNode::new("bold italic")],
			)],)]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<strong><em>bold italic</em></strong>");
	}

	#[test]
	fn complex_composition() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				TextNode::new("Welcome to "),
				(Important, children![TextNode::new("beet")]),
				TextNode::new(", the "),
				(Emphasize, children![TextNode::new("best")]),
				TextNode::new(" framework!"),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq(
			"Welcome to <strong>beet</strong>, the <em>best</em> framework!",
		);
	}

	// -- Links --

	#[test]
	fn link_without_title() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				Link::new("https://example.com").with_text("click here"),
			]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<a href=\"https://example.com\">click here</a>");
	}

	#[test]
	fn link_with_title() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				Link::new("https://example.com")
					.with_title("Example Site")
					.with_text("example"),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq(
			"<a href=\"https://example.com\" title=\"Example Site\">example</a>",
		);
	}

	#[test]
	fn important_link() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Important, children![
				Link::new("https://example.com").with_text("important link"),
			])]))
			.id();
		render_card(&mut world, entity).xpect_eq(
			"<a href=\"https://example.com\"><strong>important link</strong></a>",
		);
	}

	// -- Structural elements --

	#[test]
	fn heading_renders() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, Heading1::with_text("Hello World")))
			.id();
		render_card(&mut world, entity).xpect_eq("<h1>Hello World</h1>");
	}

	#[test]
	fn heading2_renders() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				Heading1::with_text("Outer"),
				Heading2::with_text("Inner"),
			]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<h1>Outer</h1><h2>Inner</h2>");
	}

	#[test]
	fn paragraph_renders() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, Paragraph::with_text("A paragraph of text.")))
			.id();
		render_card(&mut world, entity).xpect_eq("<p>A paragraph of text.</p>");
	}

	#[test]
	fn mixed_structure() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				Heading1::with_text("Welcome"),
				Paragraph::with_text("This is the intro."),
			]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<h1>Welcome</h1><p>This is the intro.</p>");
	}

	// -- CardTool boundaries --

	#[test]
	fn respects_card_boundary() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				Paragraph::with_text("Inside card"),
				(CardTool, children![Paragraph::with_text(
					"Inside nested card"
				)]),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq("<p>Inside card</p>");
	}

	// -- Thematic break --

	#[test]
	fn thematic_break() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				Paragraph::with_text("above"),
				(ThematicBreak,),
				Paragraph::with_text("below"),
			]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<p>above</p><hr /><p>below</p>");
	}

	// -- Block quote --

	#[test]
	fn block_quote() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(BlockQuote, children![
				Paragraph::with_text("quoted text"),
			])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<blockquote><p>quoted text</p></blockquote>");
	}

	// -- Code block --

	#[test]
	fn code_block_no_lang() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(CodeBlock::plain(), children![
				TextNode::new("let x = 1;")
			],)]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<pre><code>let x = 1;</code></pre>");
	}

	#[test]
	fn code_block_with_lang() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(
				CodeBlock::with_language("rust"),
				children![TextNode::new("fn main() {}")],
			)]))
			.id();
		render_card(&mut world, entity).xpect_eq(
			"<pre><code class=\"language-rust\">fn main() {}</code></pre>",
		);
	}

	// -- Lists --

	#[test]
	fn unordered_list() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(ListMarker::unordered(), children![
				(ListItem, children![TextNode::new("one")]),
				(ListItem, children![TextNode::new("two")]),
			])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<ul><li>one</li><li>two</li></ul>");
	}

	#[test]
	fn ordered_list() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(ListMarker::ordered(1), children![
				(ListItem, children![TextNode::new("first")]),
				(ListItem, children![TextNode::new("second")]),
			])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<ol><li>first</li><li>second</li></ol>");
	}

	#[test]
	fn ordered_list_custom_start() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(ListMarker::ordered(5), children![
				(ListItem, children![TextNode::new("fifth")]),
			])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<ol start=\"5\"><li>fifth</li></ol>");
	}

	// -- HTML escaping --

	#[test]
	fn escapes_html_entities() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![TextNode::new(
				"<script>alert('xss')</script>"
			)]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
	}

	#[test]
	fn escapes_ampersands() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![TextNode::new("A & B")]))
			.id();
		render_card(&mut world, entity).xpect_eq("A &amp; B");
	}

	// -- Container-style inline markers (parser pattern) --

	#[test]
	fn important_container() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![
				TextNode::new("hello "),
				(Important, children![TextNode::new("bold")]),
				TextNode::new(" text"),
			])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<p>hello <strong>bold</strong> text</p>");
	}

	#[test]
	fn nested_containers() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![(
				Important,
				children![(Emphasize, children![TextNode::new("bold italic")])],
			)])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<p><strong><em>bold italic</em></strong></p>");
	}

	#[test]
	fn link_container() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![
				Link::new("https://example.com").with_text("click here"),
			])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<p><a href=\"https://example.com\">click here</a></p>");
	}

	#[test]
	fn important_link_container() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![(
				Important,
				children![
					Link::new("https://example.com")
						.with_text("important link"),
				],
			)])]))
			.id();
		render_card(&mut world, entity).xpect_eq(
			"<p><a href=\"https://example.com\"><strong>important link</strong></a></p>",
		);
	}

	#[test]
	fn strikethrough_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				TextNode::new("this is "),
				(Strikethrough, children![TextNode::new("deleted")]),
			]))
			.id();
		render_card(&mut world, entity).xpect_eq("this is <del>deleted</del>");
	}

	#[test]
	fn hard_break() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![
				TextNode::new("line one"),
				(HardBreak,),
				TextNode::new("line two"),
			])]))
			.id();
		render_card(&mut world, entity)
			.xpect_eq("<p>line one<br />line two</p>");
	}

	#[test]
	fn kitchen_sink_snapshot() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, mdx!(
				"# Welcome to Beet\n\nThis is a **bold** and *italic* intro.\n\n## Features\n\n- Fast\n- Cross-platform\n- [Documentation](https://example.com)\n\n---\n\n> A block quote with *emphasis*.\n\n```rust\nfn main() {}\n```"
			)))
			.id();
		render_card(&mut world, entity).xpect_snapshot();
	}
}
