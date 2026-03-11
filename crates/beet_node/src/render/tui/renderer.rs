//! Render entity trees to a ratatui [`Buffer`] via the
//! [`NodeVisitor`] pattern.
//!
//! [`RatatuiRenderer`] walks an entity tree and writes styled text
//! directly into a ratatui [`Buffer`], advancing a cursor [`Rect`]
//! downward as content is emitted. Element names are matched to
//! determine block/inline behavior and styling.
//!
//! # Usage
//!
//! ```ignore
//! use beet_node::prelude::*;
//! use beet_core::prelude::*;
//! use ratatui::buffer::Buffer;
//! use ratatui::prelude::Rect;
//!
//! fn render(walker: NodeWalker, entity: Entity) {
//!     let area = Rect::new(0, 0, 80, 24);
//!     let mut buf = Buffer::empty(area);
//!     let mut renderer = RatatuiRenderer::new(area, &mut buf);
//!     walker.walk(&mut renderer, entity);
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
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets;
use ratatui::widgets::Widget;

/// Visitor-based TUI renderer.
///
/// Writes styled content into a ratatui [`Buffer`], consuming
/// vertical space from `area` as each block-level element is
/// rendered. Element names are matched to apply appropriate styling
/// and layout.
pub struct RatatuiRenderer<'buf> {
	/// Remaining drawable area; shrinks as content is emitted.
	area: Rect,
	/// The ratatui buffer to write into.
	buf: &'buf mut Buffer,
	/// Style stack pushed by block-level elements like headings.
	nesting_stack: Vec<Style>,
	/// Current accumulated spans with their owning entity for
	/// per-span hit-testing.
	spans: Vec<(Span<'static>, Option<Entity>)>,
	/// Whether we are inside a list item collecting text.
	in_list_item: bool,
	/// Whether we are inside a code block (pre).
	in_code_block: bool,
	/// Rendering configuration.
	config: TuiConfig,
	/// Maps terminal cell positions to entities for input hit-testing.
	span_map: TuiSpanMap,
	/// Stack of active element names for leave_element matching.
	element_stack: Vec<String>,
	/// Current heading level (0 when not in a heading).
	heading_level: u8,
	/// List nesting stack for bullet/number tracking.
	list_stack: Vec<TuiListContext>,
	/// Pending link href for underline styling.
	in_link: bool,
	/// Whether we are inside an image element.
	in_image: bool,
	/// Accumulated image src for rendering on leave.
	image_src: Option<String>,
}

impl<'buf> RatatuiRenderer<'buf> {
	/// Create a new TUI renderer targeting the given area and buffer.
	pub fn new(area: Rect, buf: &'buf mut Buffer) -> Self {
		Self {
			area,
			buf,
			nesting_stack: Vec::new(),
			spans: Vec::new(),
			in_list_item: false,
			in_code_block: false,
			config: TuiConfig::default(),
			span_map: TuiSpanMap::default(),
			element_stack: Vec::new(),
			heading_level: 0,
			list_stack: Vec::new(),
			in_link: false,
			in_image: false,
			image_src: None,
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
			nesting_stack: Vec::new(),
			spans: Vec::new(),
			in_list_item: false,
			in_code_block: false,
			config,
			span_map: TuiSpanMap::default(),
			element_stack: Vec::new(),
			heading_level: 0,
			list_stack: Vec::new(),
			in_link: false,
			in_image: false,
			image_src: None,
		}
	}

	/// Returns the remaining drawable area after rendering.
	pub fn remaining_area(&self) -> Rect { self.area }

	/// Flush remaining spans and return the populated [`TuiSpanMap`].
	///
	/// Must be called after walking completes to ensure orphaned
	/// text nodes (those not wrapped in a block-level element) are
	/// rendered.
	pub fn finish(mut self) -> TuiSpanMap {
		self.flush_spans();
		self.span_map
	}

	/// Current composite style from the stack.
	fn current_style(&self) -> Style {
		self.nesting_stack.last().copied().unwrap_or_default()
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
	/// Does **not** update the span map. Callers that need hit-testing
	/// should use [`Self::flush_spans`] or map areas manually.
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

	/// Resolve inline modifiers (bold, italic, code, link) from
	/// the current element nesting into a ratatui [`Style`].
	fn inline_style_for_element(&self, name: &str) -> Style {
		let mut style = Style::new();
		match name {
			"strong" | "b" => {
				style = style.add_modifier(Modifier::BOLD);
			}
			"em" | "i" => {
				style = style.add_modifier(Modifier::ITALIC);
			}
			"del" | "s" => {
				style = style.add_modifier(Modifier::CROSSED_OUT);
			}
			"code" if !self.in_code_block => {
				style = style.bg(Color::DarkGray);
			}
			"a" => {
				style = style
					.add_modifier(Modifier::UNDERLINED)
					.fg(self.config.link_fg);
			}
			_ => {}
		}
		style
	}

	/// Extract the heading level from an element name like `h1`, `h2`, etc.
	fn parse_heading_level(name: &str) -> Option<u8> {
		if name.starts_with('h') && name.len() == 2 {
			name[1..]
				.parse::<u8>()
				.ok()
				.filter(|level| *level >= 1 && *level <= 6)
		} else {
			None
		}
	}

	/// Whether an element name is a block-level element for TUI layout.
	fn is_block_element(name: &str) -> bool {
		matches!(
			name,
			"p" | "div"
				| "section" | "article"
				| "aside" | "nav"
				| "header" | "footer"
				| "main" | "blockquote"
				| "pre" | "ul"
				| "ol" | "li"
				| "table" | "thead"
				| "tbody" | "tr"
				| "th" | "td"
				| "h1" | "h2"
				| "h3" | "h4"
				| "h5" | "h6"
				| "hr" | "details"
				| "summary" | "figure"
				| "figcaption"
				| "dl" | "dt"
				| "dd"
		)
	}

	/// Find an attribute value by key in an attrs slice.
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


impl NodeVisitor for RatatuiRenderer<'_> {
	fn visit_element(
		&mut self,
		cx: &VisitContext,
		element: &Element,
		attrs: Vec<(Entity, &Attribute, &Value)>,
	) {
		let name = element.name().to_ascii_lowercase();
		self.element_stack.push(name.clone());

		match name.as_str() {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				let level = Self::parse_heading_level(&name).unwrap_or(3);
				self.heading_level = level;
				let style = match level {
					1 => {
						self.advance_lines(self.config.h1_gap_before);
						self.config.h1_style
					}
					2 => self.config.h2_style,
					_ => self.config.h3_style,
				};
				self.nesting_stack.push(style);
			}

			// ── Paragraph ──
			"p" | "div" | "section" | "article" | "nav" | "header"
			| "footer" | "main" => {
				self.nesting_stack.push(Style::new());
			}

			// ── Blockquote ──
			"blockquote" | "aside" => {
				self.advance_lines(self.config.block_quote_gap);
				let indent = self.config.block_quote_indent;
				if self.area.width > indent {
					if self.area.height > 0 {
						let bar = Span::styled(
							"│ ",
							Style::new().fg(Color::DarkGray),
						);
						let col = self.area.x;
						let row = self.area.y;
						self.buf.set_span(col, row, &bar, indent);
					}
					self.area.x = self.area.x.saturating_add(indent);
					self.area.width = self.area.width.saturating_sub(indent);
				}
				self.nesting_stack.push(Style::new().italic());
			}

			// ── Code blocks ──
			"pre" => {
				self.in_code_block = true;
				self.nesting_stack.push(Style::new().bg(Color::DarkGray));
			}

			// ── Lists ──
			"ul" => {
				self.list_stack.push(TuiListContext {
					ordered: false,
					current_number: 1,
				});
			}
			"ol" => {
				let start = Self::find_attr(&attrs, "start")
					.and_then(|val| match val {
						Value::Uint(num) => Some(*num as usize),
						Value::Int(num) => Some(*num as usize),
						_ => None,
					})
					.unwrap_or(1);
				self.list_stack.push(TuiListContext {
					ordered: true,
					current_number: start,
				});
			}
			"li" => {
				self.in_list_item = true;
				let bullet = if let Some(list) = self.list_stack.last_mut() {
					if list.ordered {
						let num = list.current_number;
						list.current_number += 1;
						format!("{num}. ")
					} else {
						self.config.bullet.clone()
					}
				} else {
					self.config.bullet.clone()
				};

				self.push_span(
					Span::styled(
						bullet,
						Style::new().fg(self.config.bullet_fg),
					),
					cx.entity,
				);
			}

			// ── Tables ──
			"table" => {}
			"thead" => {
				self.nesting_stack.push(Style::new().bold());
			}
			"tbody" => {}
			"tr" => {}
			"th" | "td" => {
				if !self.spans.is_empty() {
					self.push_span(
						Span::styled(" │ ", Style::new().fg(Color::DarkGray)),
						cx.entity,
					);
				}
			}

			// ── Thematic break ──
			"hr" => {
				self.flush_spans();
				self.render_hr();
			}

			// ── Images ──
			"img" => {
				self.in_image = true;
				let src = Self::find_attr(&attrs, "src")
					.map(|val| val.to_string())
					.unwrap_or_default();
				self.image_src = Some(src);
				self.push_span(
					Span::styled(
						"[image: ",
						Style::new().fg(Color::DarkGray).italic(),
					),
					cx.entity,
				);
			}

			// ── Links ──
			"a" => {
				self.in_link = true;
			}

			// ── Inline formatting ──
			"strong" | "b" | "em" | "i" | "del" | "s" => {
				// Style applied in visit_value via inline_style_for_element
			}

			// ── Inline code ──
			"code" => {
				// Style applied in visit_value via inline_style_for_element
			}

			// ── Line break ──
			"br" => {
				self.flush_spans();
			}

			// ── Button-like elements ──
			"button" => {
				self.push_span(
					Span::styled("[", Style::new().fg(self.config.link_fg)),
					cx.entity,
				);
			}

			// ── Generic block handling ──
			_ => {
				if Self::is_block_element(&name) {
					self.nesting_stack.push(Style::new());
				}
			}
		}
	}

	fn leave_element(&mut self, cx: &VisitContext, element: &Element) {
		let name = element.name().to_ascii_lowercase();
		self.element_stack.pop();

		match name.as_str() {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				let level = Self::parse_heading_level(&name).unwrap_or(3);
				if level == 1 && self.config.h1_centered {
					// Centered headings: render via render_text and map
					// the whole area to the heading entity since centering
					// shifts column offsets unpredictably.
					let span_entries = std::mem::take(&mut self.spans);
					if !span_entries.is_empty() {
						let (raw_spans, entities): (Vec<_>, Vec<_>) =
							span_entries.into_iter().unzip();
						let line = Line::from(raw_spans);
						let mut text = Text::from(line);
						if let Some(style) = self.nesting_stack.last() {
							text = text.style(*style);
						}
						text = text.centered();
						let before_y = self.area.y;
						let paragraph = widgets::Paragraph::new(text)
							.wrap(widgets::Wrap { trim: false });
						let line_count =
							paragraph.line_count(self.area.width) as u16;
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
						self.area.height =
							self.area.height.saturating_sub(line_count);
					}
				} else {
					self.flush_spans();
				}
				self.heading_level = 0;
				self.nesting_stack.pop();
				self.advance_lines(self.config.heading_gap_after);
			}

			// ── Paragraph ──
			"p" | "div" | "section" | "article" | "nav" | "header"
			| "footer" | "main" => {
				self.flush_spans();
				self.nesting_stack.pop();
				self.advance_lines(self.config.paragraph_gap_after);
			}

			// ── Blockquote ──
			"blockquote" | "aside" => {
				self.flush_spans();
				let indent = self.config.block_quote_indent;
				self.area.x = self.area.x.saturating_sub(indent);
				self.area.width = self.area.width.saturating_add(indent);
				self.nesting_stack.pop();
				self.advance_lines(self.config.block_quote_gap);
			}

			// ── Code blocks ──
			"pre" => {
				self.flush_spans();
				self.in_code_block = false;
				self.nesting_stack.pop();
			}

			// ── Lists ──
			"ul" | "ol" => {
				self.list_stack.pop();
			}
			"li" => {
				self.flush_spans();
				self.in_list_item = false;
			}

			// ── Tables ──
			"table" => {}
			"thead" => {
				self.flush_spans();
				self.render_hr();
				self.nesting_stack.pop();
			}
			"tbody" => {}
			"tr" => {
				self.flush_spans();
			}
			"th" | "td" => {
				// Cells are collected as spans; the row leave flushes them
			}

			// ── Links ──
			"a" => {
				self.in_link = false;
			}

			// ── Images ──
			"img" => {
				let src = self.image_src.take().unwrap_or_default();
				self.push_span(
					Span::styled(
						format!(": {src}]"),
						Style::new().fg(Color::DarkGray).italic(),
					),
					cx.entity,
				);
				self.in_image = false;
				self.flush_spans();
			}

			// ── Thematic break / line break ──
			"hr" | "br" => {
				// already handled in visit_element
			}

			// ── Button ──
			"button" => {
				self.push_span(
					Span::styled("]", Style::new().fg(self.config.link_fg)),
					cx.entity,
				);
			}

			// ── Inline formatting ──
			"strong" | "b" | "em" | "i" | "del" | "s" | "code" => {
				// Style was applied per-span in visit_value
			}

			// ── Generic block handling ──
			_ => {
				if Self::is_block_element(&name) {
					self.flush_spans();
					self.nesting_stack.pop();
				}
			}
		}
	}

	fn visit_value(&mut self, cx: &VisitContext, value: &Value) {
		let text = value.to_string();
		if text.is_empty() {
			return;
		}

		// Build style from the nesting stack and inline element context
		let mut style = self.current_style();

		// Walk the element stack to accumulate inline styles
		for elem in &self.element_stack {
			let inline = self.inline_style_for_element(elem);
			style = style.patch(inline);
		}

		// Apply link underline if inside <a>
		if self.in_link {
			style = style
				.add_modifier(Modifier::UNDERLINED)
				.fg(self.config.link_fg);
		}

		let entity = cx.entity;
		if self.in_code_block {
			// In code blocks, render line by line
			for line in text.lines() {
				self.push_span(Span::styled(line.to_string(), style), entity);
				self.flush_spans();
			}
		} else {
			self.push_span(Span::styled(text, style), entity);
		}
	}
}


impl NodeRenderer for RatatuiRenderer<'_> {
	fn render(
		&mut self,
		cx: &RenderContext,
	) -> Result<RenderOutput, RenderError> {
		cx.check_accepts(&[MediaType::Ratatui])?;
		cx.walker.walk(self, cx.entity);
		self.flush_spans();
		Ok(RenderOutput::Stateful)
	}
}


/// Render a ratatui [`Buffer`] to a plain-text string, stripping
/// trailing whitespace from each row.
///
/// Useful in tests to inspect rendered content without ANSI codes.
pub fn buffer_to_text(buf: &Buffer) -> String {
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


/// System that renders an entity tree into a ratatui [`Buffer`]
/// and returns both the buffer and span map.
pub fn tui_render_system(
	In((entity, area)): In<(Entity, Rect)>,
	walker: NodeWalker,
) -> (Buffer, TuiSpanMap) {
	let mut buf = Buffer::empty(area);
	let mut renderer = RatatuiRenderer::new(area, &mut buf);
	walker.walk(&mut renderer, entity);
	let span_map = renderer.finish();
	(buf, span_map)
}

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

/// Tracks the context for a list being rendered.
#[derive(Debug, Clone)]
struct TuiListContext {
	ordered: bool,
	current_number: usize,
}


#[cfg(test)]
mod test {
	use super::*;

	/// Helper: spawn an entity tree, walk it with `RatatuiRenderer`,
	/// return the buffer.
	fn render_to_buffer(
		world: &mut World,
		entity: Entity,
		width: u16,
		height: u16,
	) -> Buffer {
		let area = Rect::new(0, 0, width, height);
		let (buf, _span_map) = world
			.run_system_once_with(tui_render_system, (entity, area))
			.unwrap();
		buf
	}

	/// Helper: render and return both buffer and span map.
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

	fn cell_has_modifier(
		buf: &Buffer,
		col: u16,
		row: u16,
		modifier: Modifier,
	) -> bool {
		buf[(col, row)].style().add_modifier.contains(modifier)
	}

	// -- Basic rendering --

	#[test]
	fn heading_renders_text() {
		let mut world = World::new();
		let entity =
			world.spawn((Element::new("h1"), Children::default())).id();
		let text = world.spawn(Value::from("Hello")).id();
		world.entity_mut(entity).add_children(&[text]);

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		buffer_to_text(&buf).as_str().xpect_contains("Hello");
	}

	#[test]
	fn paragraph_renders_text() {
		let mut world = World::new();
		let entity = world.spawn((Element::new("p"), Children::default())).id();
		let text = world.spawn(Value::from("Body text")).id();
		world.entity_mut(entity).add_children(&[text]);

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		buffer_to_text(&buf).as_str().xpect_contains("Body text");
	}

	#[test]
	fn bold_text_has_bold_style() {
		let mut world = World::new();
		// <p><strong>bold</strong></p>
		let text_entity = world.spawn(Value::from("bold")).id();
		let strong = world
			.spawn((Element::new("strong"), Children::default()))
			.id();
		world.entity_mut(strong).add_children(&[text_entity]);
		let para = world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(para).add_children(&[strong]);

		let buf = render_to_buffer(&mut world, para, 40, 10);
		buffer_to_text(&buf).as_str().xpect_contains("bold");

		// Find the 'b' of "bold" and check it's bold
		let area = buf.area;
		let mut found_bold = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				if buf[(col, row)].symbol() == "b"
					&& cell_has_modifier(&buf, col, row, Modifier::BOLD)
				{
					found_bold = true;
				}
			}
		}
		found_bold.xpect_true();
	}

	#[test]
	fn italic_text_has_italic_style() {
		let mut world = World::new();
		let text_entity = world.spawn(Value::from("italic")).id();
		let em = world.spawn((Element::new("em"), Children::default())).id();
		world.entity_mut(em).add_children(&[text_entity]);
		let para = world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(para).add_children(&[em]);

		let buf = render_to_buffer(&mut world, para, 40, 10);
		buffer_to_text(&buf).as_str().xpect_contains("italic");

		let area = buf.area;
		let mut found_italic = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				if buf[(col, row)].symbol() == "i"
					&& cell_has_modifier(&buf, col, row, Modifier::ITALIC)
				{
					found_italic = true;
				}
			}
		}
		found_italic.xpect_true();
	}

	#[test]
	fn heading_is_bold() {
		let mut world = World::new();
		let text_entity = world.spawn(Value::from("Title")).id();
		let heading =
			world.spawn((Element::new("h1"), Children::default())).id();
		world.entity_mut(heading).add_children(&[text_entity]);

		let buf = render_to_buffer(&mut world, heading, 40, 10);

		let area = buf.area;
		let mut found_bold = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				if buf[(col, row)].symbol() == "T"
					&& cell_has_modifier(&buf, col, row, Modifier::BOLD)
				{
					found_bold = true;
				}
			}
		}
		found_bold.xpect_true();
	}

	#[test]
	fn thematic_break_renders_line() {
		let mut world = World::new();
		// Build a root with: <p>above</p><hr><p>below</p>
		let root = world.spawn(Children::default()).id();
		let above_text = world.spawn(Value::from("above")).id();
		let above_p =
			world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(above_p).add_children(&[above_text]);
		let hr = world.spawn(Element::new("hr")).id();
		let below_text = world.spawn(Value::from("below")).id();
		let below_p =
			world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(below_p).add_children(&[below_text]);
		world.entity_mut(root).add_children(&[above_p, hr, below_p]);

		let buf = render_to_buffer(&mut world, root, 40, 10);
		let text = buffer_to_text(&buf);
		text.as_str().xpect_contains("above");
		text.as_str().xpect_contains("─");
		text.as_str().xpect_contains("below");
	}

	#[test]
	fn code_has_background() {
		let mut world = World::new();
		let text_entity = world.spawn(Value::from("code_text")).id();
		let code = world
			.spawn((Element::new("code"), Children::default()))
			.id();
		world.entity_mut(code).add_children(&[text_entity]);
		let para = world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(para).add_children(&[code]);

		let buf = render_to_buffer(&mut world, para, 40, 10);
		buffer_to_text(&buf).as_str().xpect_contains("code_text");

		let area = buf.area;
		let mut found_bg = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				if buf[(col, row)].symbol() == "c"
					&& buf[(col, row)].style().bg == Some(Color::DarkGray)
				{
					found_bg = true;
				}
			}
		}
		found_bg.xpect_true();
	}

	#[test]
	fn link_text_is_underlined() {
		let mut world = World::new();
		let text_entity = world.spawn(Value::from("click")).id();
		let link = world.spawn((Element::new("a"), Children::default())).id();
		world.entity_mut(link).add_children(&[text_entity]);
		let para = world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(para).add_children(&[link]);

		let buf = render_to_buffer(&mut world, para, 40, 10);

		let area = buf.area;
		let mut found_underline = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				if buf[(col, row)].symbol() == "c"
					&& cell_has_modifier(&buf, col, row, Modifier::UNDERLINED)
				{
					found_underline = true;
				}
			}
		}
		found_underline.xpect_true();
	}

	// -- Span map tests --

	#[test]
	fn span_map_populated_for_paragraph() {
		let mut world = World::new();
		let text_entity = world.spawn(Value::from("hello")).id();
		let para = world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(para).add_children(&[text_entity]);

		let (_buf, span_map) = render_with_span_map(&mut world, para, 40, 10);
		span_map.is_empty().xpect_false();
		span_map.get(TuiPos::new(0, 0)).xpect_some();
	}

	#[test]
	fn span_map_maps_to_text_entity() {
		let mut world = World::new();
		let text_entity = world.spawn(Value::from("body")).id();
		let para = world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(para).add_children(&[text_entity]);
		let root = world.spawn(Children::default()).id();
		world.entity_mut(root).add_children(&[para]);

		let (_buf, span_map) = render_with_span_map(&mut world, root, 40, 10);
		// Per-span mapping: cells map to the text node, not the paragraph
		span_map.get(TuiPos::new(0, 0)).xpect_eq(Some(text_entity));
	}

	#[test]
	fn span_map_resolves_inline_spans() {
		let mut world = World::new();
		// <p>this is some <strong>Bold Text</strong></p>
		let plain_text = world.spawn(Value::from("this is some ")).id();
		let bold_text = world.spawn(Value::from("Bold Text")).id();
		let strong = world
			.spawn((Element::new("strong"), Children::default()))
			.id();
		world.entity_mut(strong).add_children(&[bold_text]);
		let para = world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(para).add_children(&[plain_text, strong]);
		let root = world.spawn(Children::default()).id();
		world.entity_mut(root).add_children(&[para]);

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
		let entity = world.spawn_empty().id();

		let (_buf, span_map) = render_with_span_map(&mut world, entity, 40, 10);
		span_map.is_empty().xpect_true();
	}

	#[test]
	fn bare_text_node_is_flushed() {
		let mut world = World::new();
		// A bare Value node not wrapped in any element
		let entity = world.spawn(Value::from("I should be visible")).id();

		let buf = render_to_buffer(&mut world, entity, 80, 24);
		buffer_to_text(&buf)
			.as_str()
			.xpect_contains("I should be visible");
	}

	#[test]
	fn button_renders_brackets() {
		let mut world = World::new();
		let btn_text = world.spawn(Value::from("Click me")).id();
		let button = world
			.spawn((Element::new("button"), Children::default()))
			.id();
		world.entity_mut(button).add_children(&[btn_text]);
		let para = world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(para).add_children(&[button]);

		let buf = render_to_buffer(&mut world, para, 40, 10);
		let text = buffer_to_text(&buf);
		text.as_str().xpect_contains("[Click me]");
	}

	#[test]
	fn mixed_inline_styles() {
		let mut world = World::new();
		// <p>normal <strong>bold</strong> end</p>
		let normal = world.spawn(Value::from("normal ")).id();
		let bold_text = world.spawn(Value::from("bold")).id();
		let strong = world
			.spawn((Element::new("strong"), Children::default()))
			.id();
		world.entity_mut(strong).add_children(&[bold_text]);
		let end = world.spawn(Value::from(" end")).id();
		let para = world.spawn((Element::new("p"), Children::default())).id();
		world.entity_mut(para).add_children(&[normal, strong, end]);

		let buf = render_to_buffer(&mut world, para, 40, 10);
		let text = buffer_to_text(&buf);
		text.as_str().xpect_contains("normal");
		text.as_str().xpect_contains("bold");
		text.as_str().xpect_contains("end");
	}

	// -- Markdown roundtrip tests --

	#[cfg(feature = "markdown_parser")]
	fn render_markdown(md: &str, width: u16, height: u16) -> String {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::markdown(md);
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();

		let buf = render_to_buffer(&mut world, entity, width, height);
		buffer_to_text(&buf)
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn markdown_heading_renders() {
		render_markdown("# Hello World", 40, 10)
			.as_str()
			.xpect_contains("Hello World");
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn markdown_paragraph_renders() {
		render_markdown("body text", 40, 10)
			.as_str()
			.xpect_contains("body text");
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn markdown_bold_is_styled() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::markdown("**bold**");
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();

		let buf = render_to_buffer(&mut world, entity, 40, 10);
		let area = buf.area;
		let mut found_bold = false;
		for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				if buf[(col, row)].symbol() == "b"
					&& cell_has_modifier(&buf, col, row, Modifier::BOLD)
				{
					found_bold = true;
				}
			}
		}
		found_bold.xpect_true();
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn markdown_hr_renders() {
		render_markdown("above\n\n---\n\nbelow", 40, 10)
			.as_str()
			.xpect_contains("─");
	}

	#[cfg(feature = "markdown_parser")]
	#[test]
	fn markdown_unordered_list() {
		render_markdown("- alpha\n- beta", 40, 10)
			.as_str()
			.xpect_contains("• alpha")
			.xpect_contains("• beta");
	}
}
