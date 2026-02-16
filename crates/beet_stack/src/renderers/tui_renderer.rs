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

/// Visitor-based TUI renderer.
///
/// Writes styled content into a ratatui [`Buffer`], consuming
/// vertical space from `area` as each block-level element is
/// rendered. Inline formatting from [`InlineStyle`] is translated
/// to ratatui [`Style`] modifiers.
pub struct TuiRenderer<'buf> {
	/// Remaining drawable area; shrinks as content is emitted.
	area: Rect,
	/// The ratatui buffer to write into.
	buf: &'buf mut Buffer,
	/// Style stack pushed by block-level elements like headings.
	style_stack: Vec<Style>,
	/// Current accumulated spans for the active line/block.
	spans: Vec<Span<'static>>,
	/// Whether we are inside a code block.
	in_code_block: bool,
	/// Tracks list nesting: `(ordered, start, current_index)`.
	list_stack: Vec<ListCtx>,
	/// Whether we are inside a list item collecting text.
	in_list_item: bool,
	/// Current heading level (0 = not in a heading).
	heading_level: u8,
}

struct ListCtx {
	ordered: bool,
	start: u64,
	current_index: u64,
}

impl<'buf> TuiRenderer<'buf> {
	/// Create a new TUI renderer targeting the given area and buffer.
	pub fn new(area: Rect, buf: &'buf mut Buffer) -> Self {
		Self {
			area,
			buf,
			style_stack: Vec::new(),
			spans: Vec::new(),
			in_code_block: false,
			list_stack: Vec::new(),
			in_list_item: false,
			heading_level: 0,
		}
	}

	/// Returns the remaining drawable area after rendering.
	pub fn remaining_area(&self) -> Rect { self.area }

	/// Convert [`InlineStyle`] markers to a ratatui [`Style`].
	fn style_from_inline(style: &InlineStyle) -> Style {
		let mut result = Style::new();
		if style.important {
			result = result.bold();
		}
		if style.emphasize {
			result = result.italic();
		}
		if style.code {
			result = result.bg(Color::DarkGray);
		}
		if style.strikethrough {
			result = result.crossed_out();
		}
		if style.link.is_some() {
			result = result.underlined().fg(Color::Cyan);
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
			Line::from(Span::styled(rule, Style::new().fg(Color::DarkGray)));
		let text = Text::from(line);
		self.render_text(text);
	}
}

impl CardVisitor for TuiRenderer<'_> {
	// -- Block-level visit --

	fn visit_heading(
		&mut self,
		_entity: Entity,
		heading: &Heading,
	) -> ControlFlow<()> {
		self.heading_level = heading.level();
		let mut style = Style::new().bold();
		if heading.level() == 1 {
			// h1: add a gap above
			self.area.y = self.area.y.saturating_add(1);
			self.area.height = self.area.height.saturating_sub(1);
			style = style.fg(Color::Yellow);
		} else if heading.level() == 2 {
			style = style.fg(Color::Cyan);
		}
		self.style_stack.push(style);
		ControlFlow::Continue(())
	}

	fn visit_paragraph(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.style_stack.push(Style::new());
		ControlFlow::Continue(())
	}

	fn visit_block_quote(&mut self, _entity: Entity) -> ControlFlow<()> {
		// Indent the area for block quotes
		let indent: u16 = 2;
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
		_entity: Entity,
		_code_block: &CodeBlock,
	) -> ControlFlow<()> {
		self.in_code_block = true;
		self.style_stack.push(Style::new().bg(Color::DarkGray));
		ControlFlow::Continue(())
	}

	fn visit_list(
		&mut self,
		_entity: Entity,
		list_marker: &ListMarker,
	) -> ControlFlow<()> {
		self.list_stack.push(ListCtx {
			ordered: list_marker.ordered,
			start: list_marker.start.unwrap_or(1),
			current_index: 0,
		});
		ControlFlow::Continue(())
	}

	fn visit_list_item(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.in_list_item = true;

		// Emit the bullet/number prefix
		let bullet = if let Some(ctx) = self.list_stack.last() {
			if ctx.ordered {
				let num = ctx.start + ctx.current_index;
				format!("{num}. ")
			} else {
				"• ".to_string()
			}
		} else {
			"• ".to_string()
		};

		self.spans
			.push(Span::styled(bullet, Style::new().fg(Color::DarkGray)));
		ControlFlow::Continue(())
	}

	fn visit_table(
		&mut self,
		_entity: Entity,
		_table: &Table,
	) -> ControlFlow<()> {
		// Tables are complex; render a placeholder for now
		ControlFlow::Continue(())
	}

	fn visit_table_head(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.style_stack.push(Style::new().bold());
		ControlFlow::Continue(())
	}

	fn visit_table_row(&mut self, _entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	fn visit_table_cell(&mut self, _entity: Entity) -> ControlFlow<()> {
		// Add a separator between cells
		if !self.spans.is_empty() {
			self.spans
				.push(Span::styled(" │ ", Style::new().fg(Color::DarkGray)));
		}
		ControlFlow::Continue(())
	}

	fn visit_thematic_break(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.flush_spans();
		self.render_hr();
		ControlFlow::Continue(())
	}

	fn visit_image(
		&mut self,
		_entity: Entity,
		image: &Image,
	) -> ControlFlow<()> {
		// Render image as alt text placeholder
		let alt = image.title.as_deref().unwrap_or(&image.src);
		self.spans.push(Span::styled(
			format!("[image: {alt}]"),
			Style::new().fg(Color::DarkGray).italic(),
		));
		self.flush_spans();
		ControlFlow::Break(())
	}

	fn visit_footnote_definition(
		&mut self,
		_entity: Entity,
		footnote_def: &FootnoteDefinition,
	) -> ControlFlow<()> {
		self.spans.push(Span::styled(
			format!("[^{}]: ", footnote_def.label),
			Style::new().fg(Color::DarkGray),
		));
		ControlFlow::Continue(())
	}

	fn visit_math_display(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.in_code_block = true;
		self.style_stack.push(Style::new().fg(Color::Magenta));
		ControlFlow::Continue(())
	}

	fn visit_html_block(
		&mut self,
		_entity: Entity,
		html_block: &HtmlBlock,
	) -> ControlFlow<()> {
		if !html_block.0.is_empty() {
			self.spans.push(Span::styled(
				html_block.0.clone(),
				Style::new().fg(Color::DarkGray),
			));
			self.flush_spans();
		}
		ControlFlow::Break(())
	}

	fn visit_button(
		&mut self,
		_entity: Entity,
		label: Option<&TextNode>,
	) -> ControlFlow<()> {
		let label_text = label.map(|text| text.as_str()).unwrap_or("button");
		let button =
			crate::prelude::widgets::Button::new(label_text.to_string());
		if self.area.height >= 3 {
			let button_area =
				Rect::new(self.area.x, self.area.y, self.area.width.min(20), 3);
			button.render(button_area, self.buf);
			self.area.y = self.area.y.saturating_add(3);
			self.area.height = self.area.height.saturating_sub(3);
		}
		ControlFlow::Break(())
	}

	// -- Inline --

	fn visit_text(
		&mut self,
		_entity: Entity,
		text: &TextNode,
		style: &InlineStyle,
	) -> ControlFlow<()> {
		let inline_style = Self::style_from_inline(style);
		let base = self.current_style().patch(inline_style);

		if self.in_code_block {
			// In code blocks, render line by line
			for line in text.as_str().lines() {
				self.spans.push(Span::styled(line.to_string(), base));
				self.flush_spans();
			}
		} else if let Some(ref link) = style.link {
			// Render as a hyperlink widget when possible
			let hyperlink = crate::prelude::widgets::Hyperlink::new(
				text.as_str().to_string(),
				link.href.clone(),
			);
			self.flush_spans();
			if self.area.height > 0 {
				let link_area =
					Rect::new(self.area.x, self.area.y, self.area.width, 1);
				hyperlink.render(link_area, self.buf);
				self.area.y = self.area.y.saturating_add(1);
				self.area.height = self.area.height.saturating_sub(1);
			}
		} else {
			self.spans
				.push(Span::styled(text.as_str().to_string(), base));
		}
		ControlFlow::Continue(())
	}

	fn visit_hard_break(&mut self, _entity: Entity) -> ControlFlow<()> {
		self.flush_spans();
		ControlFlow::Continue(())
	}

	fn visit_soft_break(&mut self, _entity: Entity) -> ControlFlow<()> {
		// Soft breaks are rendered as spaces in TUI
		self.spans.push(Span::raw(" "));
		ControlFlow::Continue(())
	}

	fn visit_footnote_ref(
		&mut self,
		_entity: Entity,
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
		_entity: Entity,
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
		_entity: Entity,
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

	fn leave_heading(&mut self, _entity: Entity) {
		// Flush heading spans with the heading style applied to the
		// entire line, then center h1.
		let spans = std::mem::take(&mut self.spans);
		if !spans.is_empty() {
			let line = Line::from(spans);
			let mut text = Text::from(line);
			if let Some(style) = self.style_stack.last() {
				text = text.style(*style);
			}
			if self.heading_level == 1 {
				text = text.centered();
			}
			self.render_text(text);
		}
		self.heading_level = 0;
		self.style_stack.pop();
	}

	fn leave_paragraph(&mut self, _entity: Entity) {
		self.flush_spans();
		self.style_stack.pop();
	}

	fn leave_block_quote(&mut self, _entity: Entity) {
		self.flush_spans();
		// Restore the area indentation
		let indent: u16 = 2;
		self.area.x = self.area.x.saturating_sub(indent);
		self.area.width = self.area.width.saturating_add(indent);
		self.style_stack.pop();
	}

	fn leave_code_block(&mut self, _entity: Entity) {
		self.flush_spans();
		self.in_code_block = false;
		self.style_stack.pop();
	}

	fn leave_list(&mut self, _entity: Entity) { self.list_stack.pop(); }

	fn leave_list_item(&mut self, _entity: Entity) {
		self.flush_spans();
		self.in_list_item = false;
		if let Some(ctx) = self.list_stack.last_mut() {
			ctx.current_index += 1;
		}
	}

	fn leave_table(&mut self, _entity: Entity) {}

	fn leave_table_head(&mut self, _entity: Entity) {
		self.flush_spans();
		// Render a separator line
		self.render_hr();
		self.style_stack.pop();
	}

	fn leave_table_row(&mut self, _entity: Entity) { self.flush_spans(); }

	fn leave_table_cell(&mut self, _entity: Entity) {
		// Cells are collected as spans; the row leave flushes them
	}
}


#[cfg(test)]
mod test {
	use super::*;

	/// Helper: create a buffer and render a card into it.
	fn render_to_buffer(
		world: &mut World,
		entity: Entity,
		width: u16,
		height: u16,
	) -> Buffer {
		let area = Rect::new(0, 0, width, height);
		let mut buf = Buffer::empty(area);
		world
			.run_system_once_with(
				|In((entity, area)): In<(Entity, Rect)>, walker: CardWalker| {
					let mut inner_buf = Buffer::empty(area);
					let mut renderer = TuiRenderer::new(area, &mut inner_buf);
					walker.walk_card(&mut renderer, entity);
					inner_buf
				},
				(entity, area),
			)
			.unwrap_or_else(|_| {
				// Fallback: render directly outside the system
				buf.clone()
			})
			.clone_into(&mut buf);
		buf
	}

	/// Extract all text from a buffer as a single string (trimmed of
	/// trailing spaces per line).
	fn buffer_text(buf: &Buffer) -> String {
		let area = buf.area;
		let mut lines = Vec::new();
		for row in area.y..area.y + area.height {
			let mut line = String::new();
			for col in area.x..area.x + area.width {
				line.push_str(buf[(col, row)].symbol());
			}
			lines.push(line.trim_end().to_string());
		}
		// Trim trailing empty lines
		while lines.last().is_some_and(|line| line.is_empty()) {
			lines.pop();
		}
		lines.join("\n")
	}

	/// Check that a buffer cell at a given position has a specific
	/// style modifier.
	fn cell_is_bold(buf: &Buffer, col: u16, row: u16) -> bool {
		use ratatui::style::Modifier;
		buf[(col, row)].modifier.contains(Modifier::BOLD)
	}

	fn cell_is_italic(buf: &Buffer, col: u16, row: u16) -> bool {
		use ratatui::style::Modifier;
		buf[(col, row)].modifier.contains(Modifier::ITALIC)
	}

	// -- Basic rendering --

	#[test]
	fn heading_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Heading1, children![TextNode::new(
				"Hello World"
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 80, 24);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("Hello World");
	}

	#[test]
	fn paragraph_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![TextNode::new(
				"body text"
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 80, 24);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("body text");
	}

	#[test]
	fn bold_text_has_bold_style() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![(
				Important,
				TextNode::new("bold"),
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 80, 24);
		let text = buffer_text(&buf);
		text.as_str().xpect_contains("bold");

		// Find the 'b' in "bold" and check its style
		let area = buf.area;
		let mut found_bold = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				if buf[(col, row)].symbol() == "b" {
					if cell_is_bold(&buf, col, row) {
						found_bold = true;
					}
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
				TextNode::new("italic"),
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 80, 24);
		let text = buffer_text(&buf);
		text.xpect_contains("italic");

		// Find the 'i' in "italic" and check its style
		let area = buf.area;
		let mut found_italic = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				if buf[(col, row)].symbol() == "i" {
					if cell_is_italic(&buf, col, row) {
						found_italic = true;
					}
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
				(Important, TextNode::new("bold")),
				TextNode::new(" words"),
			])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 80, 24);
		let rendered = buffer_text(&buf);
		rendered.as_str().xpect_contains("normal");
		rendered.as_str().xpect_contains("bold");
		rendered.as_str().xpect_contains("words");
	}

	#[test]
	fn heading_is_bold() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Heading2, children![TextNode::new(
				"Title"
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 80, 24);
		let rendered = buffer_text(&buf);
		rendered.as_str().xpect_contains("Title");

		// Heading text should be bold
		let buf_area = buf.area;
		let mut found_bold = false;
		for row in buf_area.y..buf_area.y + buf_area.height {
			for col in buf_area.x..buf_area.x + buf_area.width {
				if buf[(col, row)].symbol() == "T" {
					if cell_is_bold(&buf, col, row) {
						found_bold = true;
					}
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
				Paragraph::with_text("above"),
				(ThematicBreak,),
				Paragraph::with_text("below"),
			]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let rendered = buffer_text(&buf);
		rendered.as_str().xpect_contains("above");
		rendered.as_str().xpect_contains("below");
		// The horizontal rule should contain box-drawing characters
		rendered.as_str().xpect_contains("─");
	}

	#[test]
	fn code_has_background() {
		let mut world = World::new();
		let entity = world
			.spawn((Card, children![(Paragraph, children![(
				Code,
				TextNode::new("code"),
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 80, 24);
		let rendered = buffer_text(&buf);
		rendered.as_str().xpect_contains("code");

		// Find the 'c' in "code" and check it has a background
		let buf_area = buf.area;
		let mut found_bg = false;
		for row in buf_area.y..buf_area.y + buf_area.height {
			for col in buf_area.x..buf_area.x + buf_area.width {
				if buf[(col, row)].symbol() == "c" {
					if buf[(col, row)].bg == Color::DarkGray {
						found_bg = true;
					}
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
				(Paragraph, children![TextNode::new("visible")]),
				(Card, children![(Paragraph, children![TextNode::new(
					"hidden"
				)])]),
			]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 80, 24);
		let rendered = buffer_text(&buf);
		rendered.as_str().xpect_contains("visible");
		// The nested card content should not appear
		let has_hidden = rendered.contains("hidden");
		has_hidden.xpect_false();
	}
}
