//! Render content trees to a ratatui [`Buffer`] via the
//! [`CardVisitor`] pattern.
//!
//! [`TuiRenderer`] walks a card's entity tree and writes styled text
//! directly into a ratatui [`Buffer`], advancing a cursor [`Rect`]
//! downward as content is emitted. This replaces the older
//! per-component [`Renderable`] trait with a unified visitor.
//!
//! # Usage
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//! use ratatui::buffer::Buffer;
//! use ratatui::prelude::Rect;
//!
//! fn render(walker: CardWalker, entity: Entity) {
//!     let area = Rect::new(0, 0, 80, 24);
//!     let mut buf = Buffer::empty(area);
//!     let mut renderer = TuiRenderer::new(area, &mut buf);
//!     walker.walk_card(&mut renderer, entity);
//! }
//! ```
//!
//! # Configuration
//!
//! Use [`TuiConfig`] to customize rendering behavior such as
//! heading gaps, block quote indentation, and list bullet style.
use crate::prelude::*;
use beet_core::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
use ratatui::style::Color;
use ratatui::style::Style;

use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets;
use ratatui::widgets::Widget;
use std::ops::ControlFlow;


/// Configuration for the TUI renderer.
///
/// Controls spacing, indentation, and styling behavior. Sensible
/// defaults are provided via [`Default`].
#[derive(Debug, Clone)]
pub struct TuiConfig {
	/// Number of blank lines before an h1 heading.
	pub h1_gap_before: u16,
	/// Number of blank lines after any heading.
	pub heading_gap_after: u16,
	/// Number of blank lines after a paragraph.
	pub paragraph_gap_after: u16,
	/// Number of blank lines before and after a block quote.
	pub block_quote_gap: u16,
	/// Indentation width for block quotes (includes the `│ ` bar).
	pub block_quote_indent: u16,
	/// Style applied to h1 headings.
	pub h1_style: Style,
	/// Style applied to h2 headings.
	pub h2_style: Style,
	/// Style applied to h3+ headings.
	pub h3_style: Style,
	/// Whether h1 headings are centered.
	pub h1_centered: bool,
	/// Foreground color for link text.
	pub link_fg: Color,
	/// Foreground color for the horizontal rule.
	pub hr_fg: Color,
	/// Unordered list bullet string.
	pub bullet: String,
	/// Foreground color for bullet / list number prefixes.
	pub bullet_fg: Color,
}

impl Default for TuiConfig {
	fn default() -> Self {
		Self {
			h1_gap_before: 2,
			heading_gap_after: 0,
			paragraph_gap_after: 0,
			block_quote_gap: 1,
			block_quote_indent: 2,
			h1_style: Style::new().bold().fg(Color::Yellow),
			h2_style: Style::new().bold().fg(Color::Cyan),
			h3_style: Style::new().bold(),
			h1_centered: true,
			link_fg: Color::Cyan,
			hr_fg: Color::DarkGray,
			bullet: "• ".to_string(),
			bullet_fg: Color::DarkGray,
		}
	}
}


/// Visitor-based TUI renderer.
///
/// Writes styled content into a ratatui [`Buffer`], consuming
/// vertical space from `area` as each block-level element is
/// rendered. Inline formatting from [`InlineStyle`] is translated
/// to ratatui [`Style`] modifiers.
///
/// Traversal state like heading level, code block flag, and list
/// nesting is read from [`VisitContext`] rather than tracked locally.
pub struct TuiRenderer<'buf> {
	/// Remaining drawable area; shrinks as content is emitted.
	area: Rect,
	/// The ratatui buffer to write into.
	buf: &'buf mut Buffer,
	/// Style stack pushed by block-level elements like headings.
	style_stack: Vec<Style>,
	/// Current accumulated spans for the active line/block.
	spans: Vec<Span<'static>>,
	/// Whether we are inside a list item collecting text.
	in_list_item: bool,
	/// Rendering configuration.
	config: TuiConfig,
}

impl<'buf> TuiRenderer<'buf> {
	/// Create a new TUI renderer targeting the given area and buffer.
	pub fn new(area: Rect, buf: &'buf mut Buffer) -> Self {
		Self {
			area,
			buf,
			style_stack: Vec::new(),
			spans: Vec::new(),
			in_list_item: false,
			config: TuiConfig::default(),
		}
	}

	/// Create a new TUI renderer with custom configuration.
	pub fn with_config(
		area: Rect,
		buf: &'buf mut Buffer,
		config: TuiConfig,
	) -> Self {
		Self {
			area,
			buf,
			style_stack: Vec::new(),
			spans: Vec::new(),
			in_list_item: false,
			config,
		}
	}

	/// Returns the remaining drawable area after rendering.
	pub fn remaining_area(&self) -> Rect { self.area }

	/// Convert [`InlineStyle`] modifiers to a ratatui [`Style`].
	fn style_from_inline(&self, style: &InlineStyle) -> Style {
		let mut result = Style::new();
		if style.contains(InlineStyle::BOLD) {
			result = result.bold();
		}
		if style.contains(InlineStyle::ITALIC) {
			result = result.italic();
		}
		if style.contains(InlineStyle::CODE) {
			result = result.bg(Color::DarkGray);
		}
		if style.contains(InlineStyle::STRIKETHROUGH) {
			result = result.crossed_out();
		}
		if style.contains(InlineStyle::LINK) {
			result = result.underlined().fg(self.config.link_fg);
		}
		result
	}

	/// Current composite style from the stack.
	fn current_style(&self) -> Style {
		self.style_stack.last().copied().unwrap_or_default()
	}

	/// Flush accumulated spans as a wrapped paragraph, consuming
	/// vertical space from `self.area`.
	fn flush_spans(&mut self) {
		if self.spans.is_empty() {
			return;
		}

		let spans = std::mem::take(&mut self.spans);
		let line = Line::from(spans);
		let text = Text::from(line);
		self.render_text(text);
	}

	/// Render a [`Text`] block into the buffer, consuming vertical
	/// space.
	fn render_text(&mut self, text: Text<'_>) {
		if text.lines.is_empty() {
			return;
		}
		let paragraph =
			widgets::Paragraph::new(text).wrap(widgets::Wrap { trim: false });
		let line_count = paragraph.line_count(self.area.width) as u16;
		paragraph.render(self.area, self.buf);
		self.area.y = self.area.y.saturating_add(line_count);
		self.area.height = self.area.height.saturating_sub(line_count);
	}

	/// Render a horizontal rule.
	fn render_hr(&mut self) {
		if self.area.height == 0 {
			return;
		}
		let rule = "─".repeat(self.area.width as usize);
		let line =
			Line::from(Span::styled(rule, Style::new().fg(self.config.hr_fg)));
		let text = Text::from(line);
		self.render_text(text);
	}

	/// Advance the cursor by `count` blank lines.
	fn advance_lines(&mut self, count: u16) {
		self.area.y = self.area.y.saturating_add(count);
		self.area.height = self.area.height.saturating_sub(count);
	}
}

impl CardVisitor for TuiRenderer<'_> {
	// -- Block-level visit --

	fn visit_heading(
		&mut self,
		_cx: &VisitContext,
		heading: &Heading,
	) -> ControlFlow<()> {
		let style = match heading.level() {
			1 => {
				self.advance_lines(self.config.h1_gap_before);
				self.config.h1_style
			}
			2 => self.config.h2_style,
			_ => self.config.h3_style,
		};
		self.style_stack.push(style);
		ControlFlow::Continue(())
	}

	fn visit_paragraph(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.style_stack.push(Style::new());
		ControlFlow::Continue(())
	}

	fn visit_block_quote(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.advance_lines(self.config.block_quote_gap);
		let indent = self.config.block_quote_indent;
		if self.area.width > indent {
			// Render a vertical bar
			if self.area.height > 0 {
				let bar = Span::styled("│ ", Style::new().fg(Color::DarkGray));
				let col = self.area.x;
				let row = self.area.y;
				self.buf.set_span(col, row, &bar, indent);
			}
			self.area.x = self.area.x.saturating_add(indent);
			self.area.width = self.area.width.saturating_sub(indent);
		}
		self.style_stack.push(Style::new().italic());
		ControlFlow::Continue(())
	}

	fn visit_code_block(
		&mut self,
		_cx: &VisitContext,
		_code_block: &CodeBlock,
	) -> ControlFlow<()> {
		self.style_stack.push(Style::new().bg(Color::DarkGray));
		ControlFlow::Continue(())
	}

	fn visit_list(
		&mut self,
		_cx: &VisitContext,
		_list_marker: &ListMarker,
	) -> ControlFlow<()> {
		// List stack is managed by VisitContext
		ControlFlow::Continue(())
	}

	fn visit_list_item(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		self.in_list_item = true;

		// Emit the bullet/number prefix from the context's list stack
		let bullet = if let Some(list) = cx.current_list() {
			if list.ordered {
				let num = list.current_number();
				format!("{num}. ")
			} else {
				self.config.bullet.clone()
			}
		} else {
			self.config.bullet.clone()
		};

		self.spans
			.push(Span::styled(bullet, Style::new().fg(self.config.bullet_fg)));
		ControlFlow::Continue(())
	}

	fn visit_table(
		&mut self,
		_cx: &VisitContext,
		_table: &Table,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	fn visit_table_head(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.style_stack.push(Style::new().bold());
		ControlFlow::Continue(())
	}

	fn visit_table_row(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	fn visit_table_cell(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		// Add a separator between cells
		if !self.spans.is_empty() {
			self.spans
				.push(Span::styled(" │ ", Style::new().fg(Color::DarkGray)));
		}
		ControlFlow::Continue(())
	}

	fn visit_thematic_break(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.flush_spans();
		self.render_hr();
		ControlFlow::Continue(())
	}

	fn visit_image(
		&mut self,
		_cx: &VisitContext,
		_image: &Image,
	) -> ControlFlow<()> {
		self.spans.push(Span::styled(
			"[image: ",
			Style::new().fg(Color::DarkGray).italic(),
		));
		ControlFlow::Continue(())
	}

	fn visit_footnote_definition(
		&mut self,
		_cx: &VisitContext,
		footnote_def: &FootnoteDefinition,
	) -> ControlFlow<()> {
		self.spans.push(Span::styled(
			format!("[^{}]: ", footnote_def.label),
			Style::new().fg(Color::DarkGray),
		));
		ControlFlow::Continue(())
	}

	fn visit_math_display(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.style_stack.push(Style::new().fg(Color::Magenta));
		ControlFlow::Continue(())
	}

	fn visit_html_block(
		&mut self,
		_cx: &VisitContext,
		html_block: &HtmlBlock,
	) -> ControlFlow<()> {
		if !html_block.0.is_empty() {
			self.spans.push(Span::styled(
				html_block.0.clone(),
				Style::new().fg(Color::DarkGray),
			));
		}
		ControlFlow::Continue(())
	}

	fn visit_button(
		&mut self,
		_cx: &VisitContext,
		_button: &Button,
	) -> ControlFlow<()> {
		// Render buttons as link-style inline text
		self.spans
			.push(Span::styled("[", Style::new().fg(self.config.link_fg)));
		ControlFlow::Continue(())
	}

	// -- Inline --

	fn visit_text(
		&mut self,
		cx: &VisitContext,
		text: &TextNode,
	) -> ControlFlow<()> {
		let style = cx.effective_style();
		let inline_style = self.style_from_inline(&style);
		let base = self.current_style().patch(inline_style);

		if cx.in_code_block {
			// In code blocks, render line by line
			for line in text.as_str().lines() {
				self.spans.push(Span::styled(line.to_string(), base));
				self.flush_spans();
			}
		} else {
			self.spans
				.push(Span::styled(text.as_str().to_string(), base));
		}
		ControlFlow::Continue(())
	}

	fn visit_link(
		&mut self,
		_cx: &VisitContext,
		_link: &Link,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	fn visit_hard_break(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.flush_spans();
		ControlFlow::Continue(())
	}

	fn visit_soft_break(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		// Soft breaks are rendered as spaces in TUI
		self.spans.push(Span::raw(" "));
		ControlFlow::Continue(())
	}

	fn visit_footnote_ref(
		&mut self,
		_cx: &VisitContext,
		footnote_ref: &FootnoteRef,
	) -> ControlFlow<()> {
		self.spans.push(Span::styled(
			format!("[^{}]", footnote_ref.label),
			Style::new().fg(Color::Cyan),
		));
		ControlFlow::Continue(())
	}

	fn visit_html_inline(
		&mut self,
		_cx: &VisitContext,
		html_inline: &HtmlInline,
	) -> ControlFlow<()> {
		self.spans.push(Span::styled(
			html_inline.0.clone(),
			Style::new().fg(Color::DarkGray),
		));
		ControlFlow::Continue(())
	}

	fn visit_task_list_check(
		&mut self,
		_cx: &VisitContext,
		task_check: &TaskListCheck,
	) -> ControlFlow<()> {
		let symbol = if task_check.checked { "☑ " } else { "☐ " };
		self.spans.push(Span::styled(
			symbol,
			Style::new().fg(if task_check.checked {
				Color::Green
			} else {
				Color::DarkGray
			}),
		));
		ControlFlow::Continue(())
	}

	// -- Block-level leave --

	fn leave_heading(&mut self, cx: &VisitContext, _heading: &Heading) {
		// Flush heading spans with the heading style applied to the
		// entire line, then center h1.
		let heading_level = cx.heading_level();
		let spans = std::mem::take(&mut self.spans);
		if !spans.is_empty() {
			let line = Line::from(spans);
			let mut text = Text::from(line);
			if let Some(style) = self.style_stack.last() {
				text = text.style(*style);
			}
			if heading_level == 1 && self.config.h1_centered {
				text = text.centered();
			}
			self.render_text(text);
		}
		self.style_stack.pop();
		self.advance_lines(self.config.heading_gap_after);
	}

	fn leave_paragraph(&mut self, _cx: &VisitContext) {
		self.flush_spans();
		self.style_stack.pop();
		self.advance_lines(self.config.paragraph_gap_after);
	}

	fn leave_block_quote(&mut self, _cx: &VisitContext) {
		self.flush_spans();
		// Restore the area indentation
		let indent = self.config.block_quote_indent;
		self.area.x = self.area.x.saturating_sub(indent);
		self.area.width = self.area.width.saturating_add(indent);
		self.style_stack.pop();
		self.advance_lines(self.config.block_quote_gap);
	}

	fn leave_code_block(
		&mut self,
		_cx: &VisitContext,
		_code_block: &CodeBlock,
	) {
		self.flush_spans();
		self.style_stack.pop();
	}

	fn leave_list(&mut self, _cx: &VisitContext, _list_marker: &ListMarker) {
		// List stack is managed by VisitContext
	}

	fn leave_list_item(&mut self, _cx: &VisitContext) {
		self.flush_spans();
		self.in_list_item = false;
	}

	fn leave_table(&mut self, _cx: &VisitContext, _table: &Table) {}

	fn leave_table_head(&mut self, _cx: &VisitContext) {
		self.flush_spans();
		// Render a separator line
		self.render_hr();
		self.style_stack.pop();
	}

	fn leave_table_row(&mut self, _cx: &VisitContext) { self.flush_spans(); }

	fn leave_table_cell(&mut self, _cx: &VisitContext) {
		// Cells are collected as spans; the row leave flushes them
	}

	fn leave_link(&mut self, _cx: &VisitContext, _link: &Link) {}

	fn leave_image(&mut self, _cx: &VisitContext, image: &Image) {
		self.spans.push(Span::styled(
			format!(": {}]", image.src),
			Style::new().fg(Color::DarkGray).italic(),
		));
		self.flush_spans();
	}

	fn leave_html_block(
		&mut self,
		_cx: &VisitContext,
		_html_block: &HtmlBlock,
	) {
		self.flush_spans();
	}

	fn leave_button(&mut self, _cx: &VisitContext, _button: &Button) {
		self.spans
			.push(Span::styled("]", Style::new().fg(self.config.link_fg)));
	}
}


#[cfg(test)]
mod test {
	use super::*;

	/// Helper: spawn a card, walk it with `TuiRenderer`, return the
	/// buffer.
	fn render_to_buffer(
		world: &mut World,
		entity: Entity,
		width: u16,
		height: u16,
	) -> (Buffer, Rect) {
		let area = Rect::new(0, 0, width, height);

		world
			.run_system_once_with(
				move |In((entity, area)): In<(Entity, Rect)>,
				      walker: CardWalker| {
					let mut buf = Buffer::empty(area);
					let remaining = {
						let mut renderer = TuiRenderer::new(area, &mut buf);
						walker.walk_card(&mut renderer, entity);
						renderer.remaining_area()
					};
					(buf, remaining)
				},
				(entity, area),
			)
			.unwrap()
	}

	/// Extract all non-empty text from a buffer as a single string.
	fn buffer_text(buf: &Buffer) -> String {
		let area = buf.area;
		let mut result = String::new();
		for row in area.y..area.y + area.height {
			let mut line = String::new();
			for col in area.x..area.x + area.width {
				let cell = &buf[(col, row)];
				line.push_str(cell.symbol());
			}
			let trimmed = line.trim_end();
			if !trimmed.is_empty() {
				if !result.is_empty() {
					result.push('\n');
				}
				result.push_str(trimmed);
			}
		}
		result
	}

	/// Check if a cell at (col, row) has bold modifier.
	fn cell_is_bold(buf: &Buffer, col: u16, row: u16) -> bool {
		let cell = &buf[(col, row)];
		cell.style()
			.add_modifier
			.contains(ratatui::style::Modifier::BOLD)
	}

	fn cell_is_italic(buf: &Buffer, col: u16, row: u16) -> bool {
		let cell = &buf[(col, row)];
		cell.style()
			.add_modifier
			.contains(ratatui::style::Modifier::ITALIC)
	}

	fn cell_is_underlined(buf: &Buffer, col: u16, row: u16) -> bool {
		let cell = &buf[(col, row)];
		cell.style()
			.add_modifier
			.contains(ratatui::style::Modifier::UNDERLINED)
	}

	// -- Basic rendering --

	#[test]
	fn heading_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Heading1, children![TextNode::new(
				"Hello"
			)])]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("Hello");
	}

	#[test]
	fn paragraph_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![TextNode::new(
				"Body text"
			)])]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("Body text");
	}

	#[test]
	fn bold_text_has_bold_style() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![(
				Important,
				children![TextNode::new("bold")],
			)])]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("bold");

		// Find the 'b' of "bold" and check it's bold
		let area = buf.area;
		let mut found_bold = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				let cell = &buf[(col, row)];
				if cell.symbol() == "b" && cell_is_bold(&buf, col, row) {
					found_bold = true;
				}
			}
		}
		found_bold.xpect_true();
	}

	#[test]
	fn italic_text_has_italic_style() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![(
				Emphasize,
				children![TextNode::new("italic")],
			)])]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("italic");

		// Find the 'i' of "italic" and check it's italic
		let area = buf.area;
		let mut found_italic = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				let cell = &buf[(col, row)];
				if cell.symbol() == "i" && cell_is_italic(&buf, col, row) {
					found_italic = true;
				}
			}
		}
		found_italic.xpect_true();
	}

	#[test]
	fn mixed_inline_styles() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![
				TextNode::new("normal "),
				(Important, children![TextNode::new("bold")]),
				TextNode::new(" end"),
			])]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("normal");
		text.as_str().xpect_contains("bold");
		text.as_str().xpect_contains("end");
	}

	#[test]
	fn heading_is_bold() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Heading1, children![TextNode::new(
				"Title"
			)])]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);

		// h1 text should be bold (heading style)
		let area = buf.area;
		let mut found_bold = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				let cell = &buf[(col, row)];
				if cell.symbol() == "T" && cell_is_bold(&buf, col, row) {
					found_bold = true;
				}
			}
		}
		found_bold.xpect_true();
	}

	#[test]
	fn thematic_break_renders_line() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![
				(Paragraph, children![TextNode::new("above")]),
				(ThematicBreak,),
				(Paragraph, children![TextNode::new("below")]),
			]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("above");
		// The HR renders as "─" characters
		text.as_str().xpect_contains("─");
		text.as_str().xpect_contains("below");
	}

	#[test]
	fn code_has_background() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![(
				Code,
				children![TextNode::new("code_text")],
			)])]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("code_text");

		// Find a code character and check it has a background
		let area = buf.area;
		let mut found_bg = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				let cell = &buf[(col, row)];
				if cell.symbol() == "c"
					&& cell.style().bg == Some(Color::DarkGray)
				{
					found_bg = true;
				}
			}
		}
		found_bg.xpect_true();
	}

	#[test]
	fn respects_card_boundary() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![
				(Paragraph, children![TextNode::new("Inside card")]),
				(Card, children![(Paragraph, children![TextNode::new(
					"Inside nested card"
				)])]),
			]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("Inside card");
		// Nested card content should not appear
		text.as_str().xnot().xpect_contains("Inside nested card");
	}

	// -- Link rendering --

	#[test]
	fn link_renders_inline() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![
				TextNode::new("visit "),
				(Link::new("https://example.com"), children![TextNode::new(
					"example"
				)],),
				TextNode::new(" site"),
			])]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 60, 10);
		let text = buffer_text(&buf);
		// Link text should be inline, not on a separate line
		text.as_str().xpect_contains("visit");
		text.as_str().xpect_contains("example");
		text.as_str().xpect_contains("site");
	}

	#[test]
	fn link_text_is_underlined() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![(
				Link::new("https://example.com"),
				children![TextNode::new("click")],
			)])]))
			.id();

		let (buf, _) = render_to_buffer(&mut world, entity, 40, 10);

		// Find the 'c' of "click" and check it's underlined
		let area = buf.area;
		let mut found_underline = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				let cell = &buf[(col, row)];
				if cell.symbol() == "c" && cell_is_underlined(&buf, col, row) {
					found_underline = true;
				}
			}
		}
		found_underline.xpect_true();
	}
}
