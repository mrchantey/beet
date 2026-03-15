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
//!     let span_map = renderer.finish();
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
	/// Current accumulated spans with their owning entity for
	/// per-span hit-testing.
	spans: Vec<(Span<'static>, Option<Entity>)>,
	/// Whether we are inside a list item collecting text.
	in_list_item: bool,
	/// Rendering configuration.
	config: TuiConfig,
	/// Span map for content-buffer-space entity lookups.
	span_map: TuiSpanMap,
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
			span_map: TuiSpanMap::default(),
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
			span_map: TuiSpanMap::default(),
		}
	}

	/// Returns the remaining drawable area after rendering.
	pub fn remaining_area(&self) -> Rect { self.area }

	/// Flush remaining spans and return the [`TuiSpanMap`] with
	/// terminal-space coordinates.
	///
	/// Must be called after [`CardWalker::walk_card`] returns to
	/// ensure orphaned text nodes (those not wrapped in a block-level
	/// element like [`Paragraph`]) are rendered. Without this call,
	/// bare [`TextNode`] content silently produces blank output.
	pub fn finish(mut self) -> TuiSpanMap {
		self.flush_spans();
		self.span_map
	}

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

	/// Push a span associated with the given entity for hit-testing.
	fn push_span(&mut self, span: Span<'static>, entity: Entity) {
		self.spans.push((span, Some(entity)));
	}

	/// Flush accumulated spans as a wrapped paragraph, consuming
	/// vertical space from `self.area`. Each span is mapped to its
	/// owning entity in the span map at character granularity.
	fn flush_spans(&mut self) {
		if self.spans.is_empty() {
			return;
		}

		let span_entries = std::mem::take(&mut self.spans);
		let (raw_spans, entities): (Vec<Span<'static>>, Vec<Option<Entity>>) =
			span_entries.into_iter().unzip();

		let line = Line::from(raw_spans.clone());
		let text = Text::from(line);

		let before_y = self.area.y;
		let paragraph =
			widgets::Paragraph::new(text).wrap(widgets::Wrap { trim: false });
		let line_count = paragraph.line_count(self.area.width) as u16;
		paragraph.render(self.area, self.buf);

		// Map each span's cells to its entity at character granularity.
		let mut col = self.area.x;
		let mut row = before_y;
		let area_right = self.area.x.saturating_add(self.area.width);
		for (span, entity) in raw_spans.iter().zip(entities.iter()) {
			let span_width = span.width() as u16;
			if let Some(entity) = entity {
				let mut remaining = span_width;
				let mut cur_col = col;
				let mut cur_row = row;
				while remaining > 0 {
					let space = area_right.saturating_sub(cur_col);
					let chars_this_line = remaining.min(space);
					if chars_this_line > 0 {
						let span_area =
							Rect::new(cur_col, cur_row, chars_this_line, 1);
						self.span_map.set_area(span_area, *entity);
					}
					remaining = remaining.saturating_sub(chars_this_line);
					if remaining > 0 {
						cur_row += 1;
						cur_col = self.area.x;
					} else {
						cur_col += chars_this_line;
					}
				}
				col = cur_col;
				row = cur_row;
			} else {
				col += span_width;
				while col >= area_right && area_right > self.area.x {
					col = self.area.x + (col - area_right);
					row += 1;
				}
			}
		}

		self.area.y = self.area.y.saturating_add(line_count);
		self.area.height = self.area.height.saturating_sub(line_count);
	}

	/// Render a [`Text`] block into the buffer, consuming vertical space.
	///
	/// This is a low-level helper that does **not** update the span
	/// map. Callers that need hit-testing should use [`Self::flush_spans`]
	/// or map areas manually.
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

		self.push_span(
			Span::styled(bullet, Style::new().fg(self.config.bullet_fg)),
			cx.entity(),
		);
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

	fn visit_table_cell(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		// Add a separator between cells
		if !self.spans.is_empty() {
			self.push_span(
				Span::styled(" │ ", Style::new().fg(Color::DarkGray)),
				cx.entity(),
			);
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
		cx: &VisitContext,
		_image: &Image,
	) -> ControlFlow<()> {
		self.push_span(
			Span::styled("[image: ", Style::new().fg(Color::DarkGray).italic()),
			cx.entity(),
		);
		ControlFlow::Continue(())
	}

	fn visit_footnote_definition(
		&mut self,
		cx: &VisitContext,
		footnote_def: &FootnoteDefinition,
	) -> ControlFlow<()> {
		self.push_span(
			Span::styled(
				format!("[^{}]: ", footnote_def.label),
				Style::new().fg(Color::DarkGray),
			),
			cx.entity(),
		);
		ControlFlow::Continue(())
	}

	fn visit_math_display(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
		self.style_stack.push(Style::new().fg(Color::Magenta));
		ControlFlow::Continue(())
	}

	fn visit_html_block(
		&mut self,
		cx: &VisitContext,
		html_block: &HtmlBlock,
	) -> ControlFlow<()> {
		if !html_block.0.is_empty() {
			self.push_span(
				Span::styled(
					html_block.0.clone(),
					Style::new().fg(Color::DarkGray),
				),
				cx.entity(),
			);
		}
		ControlFlow::Continue(())
	}

	fn visit_button(
		&mut self,
		cx: &VisitContext,
		_button: &Button,
	) -> ControlFlow<()> {
		// Render buttons as link-style inline text
		self.push_span(
			Span::styled("[", Style::new().fg(self.config.link_fg)),
			cx.entity(),
		);
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

		let entity = cx.entity();
		if cx.in_code_block {
			// In code blocks, render line by line
			for line in text.as_str().lines() {
				self.push_span(Span::styled(line.to_string(), base), entity);
				self.flush_spans();
			}
		} else {
			self.push_span(
				Span::styled(text.as_str().to_string(), base),
				entity,
			);
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

	fn visit_soft_break(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		// Soft breaks are rendered as spaces in TUI
		self.push_span(Span::raw(" "), cx.entity());
		ControlFlow::Continue(())
	}

	fn visit_footnote_ref(
		&mut self,
		cx: &VisitContext,
		footnote_ref: &FootnoteRef,
	) -> ControlFlow<()> {
		self.push_span(
			Span::styled(
				format!("[^{}]", footnote_ref.label),
				Style::new().fg(Color::Cyan),
			),
			cx.entity(),
		);
		ControlFlow::Continue(())
	}

	fn visit_html_inline(
		&mut self,
		cx: &VisitContext,
		html_inline: &HtmlInline,
	) -> ControlFlow<()> {
		self.push_span(
			Span::styled(
				html_inline.0.clone(),
				Style::new().fg(Color::DarkGray),
			),
			cx.entity(),
		);
		ControlFlow::Continue(())
	}

	fn visit_task_list_check(
		&mut self,
		cx: &VisitContext,
		task_check: &TaskListCheck,
	) -> ControlFlow<()> {
		let symbol = if task_check.checked { "☑ " } else { "☐ " };
		self.push_span(
			Span::styled(
				symbol,
				Style::new().fg(if task_check.checked {
					Color::Green
				} else {
					Color::DarkGray
				}),
			),
			cx.entity(),
		);
		ControlFlow::Continue(())
	}

	// -- Block-level leave --

	fn leave_heading(&mut self, cx: &VisitContext, _heading: &Heading) {
		// Flush heading spans with the heading style applied to the
		// entire line, then center h1.
		let heading_level = cx.heading_level();
		if heading_level == 1 && self.config.h1_centered {
			// Centered headings: render via render_text and map the
			// whole area to the heading entity since centering shifts
			// column offsets unpredictably.
			let span_entries = std::mem::take(&mut self.spans);
			if !span_entries.is_empty() {
				let (raw_spans, entities): (Vec<_>, Vec<_>) =
					span_entries.into_iter().unzip();
				let line = Line::from(raw_spans);
				let mut text = Text::from(line);
				if let Some(style) = self.style_stack.last() {
					text = text.style(*style);
				}
				text = text.centered();
				let before_y = self.area.y;
				let paragraph = widgets::Paragraph::new(text)
					.wrap(widgets::Wrap { trim: false });
				let line_count = paragraph.line_count(self.area.width) as u16;
				paragraph.render(self.area, self.buf);
				// Map whole area; pick the first entity with a value
				if let Some(entity) = entities.iter().flatten().next() {
					let rendered = Rect::new(
						self.area.x,
						before_y,
						self.area.width,
						line_count,
					);
					self.span_map.set_area(rendered, *entity);
				}
				self.area.y = self.area.y.saturating_add(line_count);
				self.area.height = self.area.height.saturating_sub(line_count);
			}
		} else {
			self.flush_spans();
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

	fn leave_image(&mut self, cx: &VisitContext, image: &Image) {
		self.push_span(
			Span::styled(
				format!(": {}]", image.src),
				Style::new().fg(Color::DarkGray).italic(),
			),
			cx.entity(),
		);
		self.flush_spans();
	}

	fn leave_html_block(
		&mut self,
		_cx: &VisitContext,
		_html_block: &HtmlBlock,
	) {
		self.flush_spans();
	}

	fn leave_button(&mut self, cx: &VisitContext, _button: &Button) {
		self.push_span(
			Span::styled("]", Style::new().fg(self.config.link_fg)),
			cx.entity(),
		);
	}
}


pub fn tui_render_system(
	In((entity, area)): In<(Entity, Rect)>,
	walker: CardWalker,
) -> (Buffer, TuiSpanMap) {
	let mut buf = Buffer::empty(area);
	let mut renderer = TuiRenderer::new(area, &mut buf);
	walker.walk_card(&mut renderer, entity);
	let span_map = renderer.finish();
	(buf, span_map)
}


/// Render a ratatui [`Buffer`] to a plain-text string, stripping
/// trailing whitespace from each row.
///
/// Useful in tests to inspect rendered content without ANSI codes.
pub fn buffer_to_text(buf: &ratatui::buffer::Buffer) -> String {
	let area = buf.area;
	let mut output = String::new();
	for row in area.y..area.y + area.height {
		let mut line = String::new();
		for col in area.x..area.x + area.width {
			line.push_str(buf[(col, row)].symbol());
		}
		let trimmed = line.trim_end();
		if !trimmed.is_empty() || row < area.y + area.height - 1 {
			output.push_str(trimmed);
			output.push('\n');
		}
	}
	// Remove trailing empty lines
	while output.ends_with("\n\n") {
		output.pop();
	}
	output
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
	) -> Buffer {
		let area = Rect::new(0, 0, width, height);

		let (buf, _tui_area) = world
			.run_system_once_with(tui_render_system, (entity, area))
			.unwrap();
		buf
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
			.spawn((CardTool, children![(Heading1, children![TextNode::new(
				"Hello"
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_to_text(&buf);
		text.as_str().xpect_contains("Hello");
	}

	#[test]
	fn paragraph_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![
				TextNode::new("Body text")
			])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_to_text(&buf);
		text.as_str().xpect_contains("Body text");
	}

	#[test]
	fn bold_text_has_bold_style() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![(
				Important,
				children![TextNode::new("bold")],
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_to_text(&buf);
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
			.spawn((CardTool, children![(Paragraph, children![(
				Emphasize,
				children![TextNode::new("italic")],
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_to_text(&buf);
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
			.spawn((CardTool, children![(Paragraph, children![
				TextNode::new("normal "),
				(Important, children![TextNode::new("bold")]),
				TextNode::new(" end"),
			])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_to_text(&buf);
		text.as_str().xpect_contains("normal");
		text.as_str().xpect_contains("bold");
		text.as_str().xpect_contains("end");
	}

	#[test]
	fn heading_is_bold() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Heading1, children![TextNode::new(
				"Title"
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);

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
			.spawn((CardTool, children![
				(Paragraph, children![TextNode::new("above")]),
				(ThematicBreak,),
				(Paragraph, children![TextNode::new("below")]),
			]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_to_text(&buf);
		text.as_str().xpect_contains("above");
		// The HR renders as "─" characters
		text.as_str().xpect_contains("─");
		text.as_str().xpect_contains("below");
	}

	#[test]
	fn code_has_background() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![(
				Code,
				children![TextNode::new("code_text")],
			)])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_to_text(&buf);
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
			.spawn((CardTool, children![
				(Paragraph, children![TextNode::new("Inside card")]),
				(CardTool, children![(Paragraph, children![TextNode::new(
					"Inside nested card"
				)])]),
			]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let text = buffer_to_text(&buf);
		text.as_str().xpect_contains("Inside card");
		// Nested card content should not appear
		text.as_str().xnot().xpect_contains("Inside nested card");
	}

	// -- Link rendering --

	#[test]
	fn link_renders_inline() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![
				TextNode::new("visit "),
				Link::new("https://example.com").with_text("example"),
				TextNode::new(" site"),
			])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 60, 10);
		let text = buffer_to_text(&buf);
		// Link text should be inline, not on a separate line
		text.as_str().xpect_contains("visit");
		text.as_str().xpect_contains("example");
		text.as_str().xpect_contains("site");
	}

	#[test]
	fn link_text_is_underlined() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![
				Link::new("https://example.com").with_text("click"),
			])]))
			.id();

		let buf = render_to_buffer(&mut world, entity, 40, 10);

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

	// -- Span map integration tests --

	/// Helper: render a card and return both buffer and span map.
	fn render_with_span_map(
		world: &mut World,
		entity: Entity,
		width: u16,
		height: u16,
	) -> (Buffer, TuiSpanMap) {
		let area = Rect::new(0, 0, width, height);
		world
			.run_system_once_with(tui_render_system, (entity, area))
			.unwrap()
	}

	#[test]
	fn span_map_populated_for_paragraph() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![
				TextNode::new("hello")
			])]))
			.id();

		let (_buf, span_map) = render_with_span_map(&mut world, entity, 40, 10);
		span_map.is_empty().xpect_false();
		// Row 0 should map to the text node entity
		span_map.get(TuiPos::new(0, 0)).xpect_some();
	}

	#[test]
	fn span_map_maps_to_text_entity() {
		let mut world = World::new();
		let text_entity = world.spawn(TextNode::new("body")).id();
		let para_entity = world.spawn((Paragraph, children![])).id();
		world.entity_mut(para_entity).add_children(&[text_entity]);
		let root = world.spawn((CardTool, children![])).id();
		world.entity_mut(root).add_children(&[para_entity]);

		let (_buf, span_map) = render_with_span_map(&mut world, root, 40, 10);

		// Per-span mapping: cells map to the text node, not the paragraph
		span_map.get(TuiPos::new(0, 0)).xpect_eq(Some(text_entity));
	}

	#[test]
	fn span_map_separates_heading_and_paragraph() {
		let mut world = World::new();
		let heading_text = world.spawn(TextNode::new("Title")).id();
		let heading = world.spawn((Heading1, children![])).id();
		world.entity_mut(heading).add_children(&[heading_text]);
		let para_text = world.spawn(TextNode::new("Body")).id();
		let paragraph = world.spawn((Paragraph, children![])).id();
		world.entity_mut(paragraph).add_children(&[para_text]);
		let root = world.spawn(CardTool).id();
		world.entity_mut(root).add_children(&[heading, paragraph]);

		let (_buf, span_map) = render_with_span_map(&mut world, root, 40, 10);

		// Per-span mapping: cells map to text node entities
		let mut heading_row = None;
		let mut para_row = None;
		for row in 0..10 {
			match span_map.get(TuiPos::new(row, 0)) {
				Some(entity) if entity == heading_text => {
					if heading_row.is_none() {
						heading_row = Some(row);
					}
				}
				Some(entity) if entity == para_text => {
					if para_row.is_none() {
						para_row = Some(row);
					}
				}
				_ => {}
			}
		}

		heading_row.xpect_some();
		para_row.xpect_some();
		// Heading must appear before paragraph
		(heading_row.unwrap() < para_row.unwrap()).xpect_true();
	}

	#[test]
	fn span_map_button_maps_bracket_and_text() {
		let mut world = World::new();
		let btn_text = world.spawn(TextNode::new("Click me")).id();
		let button = world.spawn((Button, children![])).id();
		world.entity_mut(button).add_children(&[btn_text]);
		let paragraph = world.spawn((Paragraph, children![])).id();
		world.entity_mut(paragraph).add_children(&[button]);
		let root = world.spawn(CardTool).id();
		world.entity_mut(root).add_children(&[paragraph]);

		let (_buf, span_map) = render_with_span_map(&mut world, root, 40, 10);

		// "[" maps to the button entity
		span_map.get(TuiPos::new(0, 0)).xpect_eq(Some(button));
		// "C" of "Click me" maps to the text node entity
		span_map.get(TuiPos::new(0, 1)).xpect_eq(Some(btn_text));
	}

	#[test]
	fn span_map_resolves_inline_spans() {
		let mut world = World::new();
		let plain_text = world.spawn(TextNode::new("this is some ")).id();
		let bold_text = world.spawn(TextNode::new("Bold Text")).id();
		let bold = world.spawn((Important, children![])).id();
		world.entity_mut(bold).add_children(&[bold_text]);
		let paragraph = world.spawn((Paragraph, children![])).id();
		world
			.entity_mut(paragraph)
			.add_children(&[plain_text, bold]);
		let root = world.spawn(CardTool).id();
		world.entity_mut(root).add_children(&[paragraph]);

		let (_buf, span_map) = render_with_span_map(&mut world, root, 40, 10);

		// "this is some " is 13 chars (cols 0..12)
		span_map.get(TuiPos::new(0, 0)).xpect_eq(Some(plain_text));
		span_map.get(TuiPos::new(0, 12)).xpect_eq(Some(plain_text));
		// "Bold Text" starts at col 13
		span_map.get(TuiPos::new(0, 13)).xpect_eq(Some(bold_text));
		span_map.get(TuiPos::new(0, 21)).xpect_eq(Some(bold_text));
		// Col 22 is past all content, should be None
		span_map.get(TuiPos::new(0, 22)).xpect_eq(None);
	}

	#[test]
	fn span_map_empty_for_no_content() {
		let mut world = World::new();
		let entity = world.spawn(CardTool).id();

		let (_buf, span_map) = render_with_span_map(&mut world, entity, 40, 10);

		span_map.is_empty().xpect_true();
	}
}
