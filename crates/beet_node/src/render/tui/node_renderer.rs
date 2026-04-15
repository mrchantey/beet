//! Render entity trees to a ratatui [`Buffer`] via the
//! [`NodeVisitor`] pattern.
//!
//! [`TuiRenderer`] walks an entity tree and writes styled text
//! directly into a ratatui [`Buffer`], advancing a cursor [`Rect`]
//! downward as content is emitted. Element names are matched to
//! determine block/inline behavior and styling.
//!
//! # Usage
//!
//! ```ignore
//! use beet_node::prelude::*;
//! use beet_core::prelude::*;
//! use ratatui::prelude::Rect;
//!
//! fn render(walker: NodeWalker, entity: Entity) {
//!     let area = Rect::new(0, 0, 80, 24);
//!     let mut renderer = TuiRenderer::new(area);
//!     walker.walk(&mut renderer, entity);
//!     let (buf, span_map) = renderer.finish();
//! }
//! ```
//!
//! # Styling
//!
//! Uses [`StyleMap<TuiStyle>`] to map element names to [`TuiStyle`]
//! values that combine a ratatui [`Style`] with layout metadata such
//! as vertical spacing and justification.
use crate::prelude::Justify;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy_ratatui::RatatuiContext;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::prelude::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets;
use ratatui::widgets::Block;
use ratatui::widgets::Widget;
use std::borrow::Cow;

/// Visitor-based TUI renderer.
///
/// Writes styled content into an owned ratatui [`Buffer`], consuming
/// vertical space from `area` as each block-level element is
/// rendered. Uses a [`StyleMap<TuiStyle>`] to resolve element styles
/// and layout metadata.
#[derive(Debug, Clone, PartialEq, Eq, Component)]
#[require(TuiScrollState, TuiWidget = widget())]
#[component(on_add=on_add)]
pub struct TuiNodeRenderer {
	/// Remaining drawable area; shrinks as content is emitted.
	area: Rect,
	/// The owned ratatui buffer to write into.
	buf: Buffer,
	/// Current accumulated spans with their owning entity for
	/// per-span hit-testing.
	spans: Vec<(Span<'static>, Option<Entity>)>,
	/// List nesting stack for bullet/number tracking, shared with
	/// [`TextRenderState`].
	list_stack: Vec<ListContext>,
	/// Whether we are inside a list item collecting text.
	in_list_item: bool,
	/// Whether we are inside a code block (pre).
	in_code_block: bool,
	/// Element → style mapping with nesting support.
	style_map: StyleMap<TuiStyle>,
	/// Block elements used for fallback block detection.
	block_elements: Vec<Cow<'static, str>>,
	/// Maps terminal-space cell positions to entities for input hit-testing.
	span_map: TuiSpanMap,
	/// Terminal-space origin `(x, y)` of the inner viewport, set before reset.
	terminal_origin: (u16, u16),
	/// Current scroll offset in content rows, set before reset.
	scroll_offset: u16,
	/// Visible viewport height in terminal rows, set before reset.
	viewport_height: u16,
	/// Unordered list bullet string.
	bullet_prefix: Cow<'static, str>,
	/// Style for bullet / list number prefixes.
	bullet_prefix_style: Style,
	/// Indentation width for block quotes (includes the `│ ` bar).
	block_quote_indent: u16,
}

fn widget() -> TuiWidget {
	TuiWidget::new(Constraint::Fill(1), |cx| {
		let mut renderer = cx
			.entity
			.get::<TuiNodeRenderer>()
			.ok_or_else(|| {
				bevyhow!("entity has no TuiNodeRenderer, was it removed?")
			})?
			.clone();
		renderer.render_inner(cx.entity, cx.draw_area, cx.buffer)?;
		Ok(())
	})
}


fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.observe(mark_widget_changed);
}

fn mark_widget_changed(ev: On<NodeParsed>, mut query: Query<&mut TuiWidget>) {
	query
		.get_mut(ev.event_target())
		.map(|mut widget| {
			widget.set_changed();
		})
		// ignore missing component
		.ok();
}

/// Create a new [`TuiRenderer`], its rect will be updated
/// on each render call, via [`Self::reset`]
impl Default for TuiNodeRenderer {
	fn default() -> Self { Self::new(Rect::default()) }
}



impl NodeRenderer for TuiNodeRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<RenderOutput, RenderError> {
		cx.check_accepts(&[MediaType::Ratatui])?;
		let id = cx.entity;
		cx.world.resource_scope(
			|world: &mut World, mut context: Mut<RatatuiContext>| -> Result {
				let mut result = None;
				context
					.draw(|frame| {
						result = self
							.render_inner(
								world.entity_mut(id),
								frame.area(),
								frame.buffer_mut(),
							)
							.xsome();
					})
					.map_err(|err| {
						bevyhow!("Failed to draw to RatatuiContext: {err}")
					})?;
				result.expect("was certainly set")
			},
		)?;
		Ok(RenderOutput::Stateful)
	}
}



impl TuiNodeRenderer {
	/// Create a new TUI renderer targeting the given area.
	pub fn new(area: Rect) -> Self {
		Self {
			// state
			area,
			buf: Buffer::empty(area),
			spans: Vec::new(),
			list_stack: Vec::new(),
			in_list_item: false,
			in_code_block: false,
			// settings
			style_map: default_tui_style_map(),
			block_elements: default_block_elements(),
			span_map: TuiSpanMap::default(),
			terminal_origin: (0, 0),
			scroll_offset: 0,
			viewport_height: u16::MAX,
			bullet_prefix: "• ".into(),
			bullet_prefix_style: Style::new().fg(Color::DarkGray),
			block_quote_indent: 2,
		}
	}

	/// Reset the renderer to an initial state with a new target area.
	///
	/// `terminal_origin`, `scroll_offset`, and `viewport_height` are
	/// intentionally preserved across resets — set them before calling
	/// this method.
	pub fn reset(&mut self, area: Rect) {
		self.area = area;
		self.buf = Buffer::empty(area);
		self.spans.clear();
		self.list_stack.clear();
		self.in_list_item = false;
		self.in_code_block = false;
		self.span_map = TuiSpanMap::default();
	}

	/// Override the element → style mapping.
	pub fn with_style_map(mut self, map: StyleMap<TuiStyle>) -> Self {
		self.style_map = map;
		self
	}

	/// Returns the remaining drawable area after rendering.
	pub fn remaining_area(&self) -> Rect { self.area }

	/// Returns a reference to the underlying buffer.
	pub fn buffer(&self) -> &Buffer { &self.buf }

	/// Flush remaining spans and return the buffer and [`TuiSpanMap`].
	///
	/// Must be called after walking completes to ensure orphaned
	/// text nodes (those not wrapped in a block-level element) are
	/// rendered.
	pub fn finish(mut self) -> (Buffer, TuiSpanMap) {
		self.flush_spans();
		(self.buf, self.span_map)
	}




	pub fn render_inner(
		&mut self,
		mut entity: EntityWorldMut,
		area: Rect,
		buf: &mut Buffer,
	) -> Result {
		/// Maximum height of the internal content buffer.
		///
		/// Content is rendered into a buffer this tall, then only the visible
		/// viewport portion is copied to the frame. This bounds memory usage
		/// while allowing generous scroll depth.
		const MAX_CONTENT_HEIGHT: u16 = 4096;

		// Read existing scroll state so we can preserve the offset
		// across frames.
		let mut scroll_state =
			entity.get::<TuiScrollState>().cloned().unwrap_or_default();

		// Extract the RatatuiContext so we can access both it and the
		// world independently, mirroring the draw system
		// approach. This avoids the `get_frame()` / `draw()` area
		// mismatch that caused cascading whitespace in terminals with
		// pre-existing scrollback (eg Zed).
		let id = entity.id();
		entity.world_scope(|world| -> Result {
			// Render the border block to the frame buffer.
			let block = Block::bordered();
			let inner_area = block.inner(area);
			block.render(area, buf);

			// Create an oversized content buffer so we can
			// measure the full content height before deciding
			// whether a scrollbar is needed.
			let content_area = Rect::new(
				0,
				0,
				inner_area.width,
				MAX_CONTENT_HEIGHT.min(
					inner_area.height.saturating_mul(20).max(inner_area.height),
				),
			);

			// Clamp scroll before we render so flush_spans
			// gets the correct offset.
			scroll_state.viewport_height = inner_area.height;
			scroll_state.clamp();

			// Set terminal-space context so flush_spans can
			// translate content coords → terminal coords.
			self.terminal_origin = (inner_area.x, inner_area.y);
			self.scroll_offset = scroll_state.offset;
			self.viewport_height = inner_area.height;
			self.reset(content_area);

			// Walk the entity tree, rendering into the
			// oversized content buffer.
			let mut inner_cx = RenderContext::new(id, world);
			inner_cx.walk(self);
			self.flush_spans();

			// Measure actual content height from the cursor
			// position (area.y was advanced past all content).
			let content_height =
				content_area.height.saturating_sub(self.area.height);

			// Update scroll state content height and re-clamp.
			scroll_state.content_height = content_height;
			scroll_state.clamp();

			// Copy the visible portion of the content buffer
			// into the frame buffer at the inner area position.
			for row in 0..inner_area.height {
				let src_row = row.saturating_add(scroll_state.offset);
				if src_row >= content_height {
					break;
				}
				for col in 0..inner_area.width {
					if let Some(dest) =
						buf.cell_mut((inner_area.x + col, inner_area.y + row))
					{
						*dest = self.buf[(col, src_row)].clone();
					}
				}
			}

			scroll_state.try_render(area, buf);

			Ok(())
		})?;

		entity.insert((scroll_state, std::mem::take(&mut self.span_map)));
		Ok(())
	}

	/// Current composite ratatui [`Style`] from the style map stack.
	fn current_style(&self) -> Style { self.style_map.current().style }

	/// Push a span associated with the given entity for hit-testing.
	fn push_span(&mut self, span: Span<'static>, entity: Entity) {
		self.spans.push((span, Some(entity)));
	}

	/// Flush accumulated spans as a wrapped paragraph, consuming
	/// vertical space from `self.area`. Each span is mapped to its
	/// owning entity in the span map at **terminal-space** coordinates.
	fn flush_spans(&mut self) {
		if self.spans.is_empty() {
			return;
		}
		let span_entries = std::mem::take(&mut self.spans);
		let (raw_spans, entities): (Vec<Span<'static>>, Vec<Option<Entity>>) =
			span_entries.into_iter().unzip();

		let before_y = self.area.y;
		let line_count = self.render_spans_paragraph(&raw_spans);
		self.map_spans_to_entities(&raw_spans, &entities, before_y);

		self.area.y = self.area.y.saturating_add(line_count);
		self.area.height = self.area.height.saturating_sub(line_count);
	}

	/// Build and render a wrapped paragraph from `spans` into `self.buf`,
	/// returning the number of lines consumed.
	fn render_spans_paragraph(&mut self, spans: &[Span<'static>]) -> u16 {
		let line = Line::from(spans.to_vec());
		let text = Text::from(line);
		let paragraph =
			widgets::Paragraph::new(text).wrap(widgets::Wrap { trim: false });
		let line_count = paragraph.line_count(self.area.width) as u16;
		paragraph.render(self.area, &mut self.buf);
		line_count
	}

	/// Walk `spans` and record each cell's owning entity in `self.span_map`,
	/// translating content-buffer coordinates to terminal space.
	///
	/// `start_row` is the content-buffer row at which the paragraph begins
	/// (i.e. `self.area.y` captured before rendering).
	fn map_spans_to_entities(
		&mut self,
		spans: &[Span<'static>],
		entities: &[Option<Entity>],
		start_row: u16,
	) {
		let mut col = self.area.x;
		let mut row = start_row;
		let area_right = self.area.x.saturating_add(self.area.width);
		let term_origin_y = self.terminal_origin.1 as i32;
		let term_origin_x = self.terminal_origin.0;
		let viewport_end = term_origin_y + self.viewport_height as i32;

		for (span, entity) in spans.iter().zip(entities.iter()) {
			let span_width = span.width() as u16;
			if let Some(entity) = entity {
				(col, row) = self.map_single_span(
					span_width,
					*entity,
					col,
					row,
					area_right,
					term_origin_x,
					term_origin_y,
					viewport_end,
				);
			} else {
				// Unowned span — advance the cursor without mapping.
				col += span_width;
				while col >= area_right && area_right > self.area.x {
					col = self.area.x + (col - area_right);
					row += 1;
				}
			}
		}
	}

	/// Map the cells of a single span to `entity` in `self.span_map`,
	/// handling line-wrapping within the content area.
	///
	/// Returns the updated `(col, row)` cursor position.
	#[allow(clippy::too_many_arguments)]
	fn map_single_span(
		&mut self,
		span_width: u16,
		entity: Entity,
		mut col: u16,
		mut row: u16,
		area_right: u16,
		term_origin_x: u16,
		term_origin_y: i32,
		viewport_end: i32,
	) -> (u16, u16) {
		let mut remaining = span_width;
		while remaining > 0 {
			let space = area_right.saturating_sub(col);
			let chars_this_line = remaining.min(space);
			if chars_this_line > 0 {
				// Translate content coords to terminal coords.
				let term_row =
					(row as i32) - (self.scroll_offset as i32) + term_origin_y;
				let term_col = col + term_origin_x;
				// Only emit cells visible in the viewport.
				if term_row >= term_origin_y && term_row < viewport_end {
					let span_area = Rect::new(
						term_col,
						term_row as u16,
						chars_this_line,
						1,
					);
					self.span_map.set_area(span_area, entity);
				}
			}
			remaining = remaining.saturating_sub(chars_this_line);
			if remaining > 0 {
				row += 1;
				col = self.area.x;
			} else {
				col += chars_this_line;
			}
		}
		(col, row)
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
		paragraph.render(self.area, &mut self.buf);

		self.area.y = self.area.y.saturating_add(line_count);
		self.area.height = self.area.height.saturating_sub(line_count);
	}

	/// Render a horizontal rule using the current style.
	fn render_hr(&mut self) {
		if self.area.height == 0 {
			return;
		}
		let style = self.current_style();
		let rule = "─".repeat(self.area.width as usize);
		let line = Line::from(Span::styled(rule, style));
		let text = Text::from(line);
		self.render_text(text);
	}

	/// Advance the cursor by `count` blank lines.
	fn advance_lines(&mut self, count: u16) {
		self.area.y = self.area.y.saturating_add(count);
		self.area.height = self.area.height.saturating_sub(count);
	}

	/// Whether an element name is block-level, using the shared
	/// [`default_block_elements`] list.
	fn is_block_element(&self, name: &str) -> bool {
		self.block_elements.iter().any(|el| el.as_ref() == name)
	}
}

impl NodeVisitor for TuiNodeRenderer {
	fn visit_element(&mut self, cx: &VisitContext, view: ElementView) {
		let name = view.tag();

		// Push the style map entry for this element (handles nesting).
		let tui_style = self.style_map.push(name);

		// Apply lines_before from TuiStyle for all elements.
		self.advance_lines(tui_style.lines_before);

		match name {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {}

			// ── Paragraph ──
			"p" | "div" | "section" | "article" | "nav" | "header"
			| "footer" | "main" => {
				// style already pushed
			}

			// ── Blockquote ──
			"blockquote" | "aside" => {
				let indent = self.block_quote_indent;
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
			}

			// ── Code blocks ──
			"pre" => {
				self.in_code_block = true;
			}

			// ── Lists ──
			"ul" => {
				self.list_stack.push(ListContext::Unordered);
			}
			"ol" => {
				let start = view.try_as::<OrderedListView>().unwrap().start;
				self.list_stack.push(ListContext::Ordered(start));
			}
			"li" => {
				self.in_list_item = true;
				let bullet = match self.list_stack.last_mut() {
					Some(ListContext::Ordered(num)) => {
						let prefix = format!("{num}. ");
						*num += 1;
						prefix.into()
					}
					_ => self.bullet_prefix.clone(),
				};

				self.push_span(
					Span::styled(bullet, self.bullet_prefix_style),
					cx.entity,
				);
			}

			// ── Tables ──
			"table" => {}
			"thead" => {
				// style already pushed (bold via style map)
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

			// ── Thematic break (void element) ──
			"hr" => {
				self.flush_spans();
				self.render_hr();
			}

			// ── Images (void element) ──
			"img" => {
				let src = view.attribute_string("src");
				let alt = view
					.attribute("alt")
					.map(|val| val.to_string())
					.unwrap_or_else(|| "image".to_string());
				let label = if src.is_empty() {
					format!("[{alt}]")
				} else {
					format!("[{alt}: {src}]")
				};
				self.push_span(Span::styled(label, tui_style.style), cx.entity);
				self.flush_spans();
			}

			// ── Line break (void element) ──
			"br" => {
				self.flush_spans();
			}

			// ── Links ──
			"a" => {
				// Style applied via style map
			}

			// ── Inline formatting ──
			"strong" | "b" | "em" | "i" | "del" | "s" => {
				// Style applied in visit_value via style map
			}

			// ── Inline code ──
			"code" => {
				// Style applied in visit_value via style map
			}

			// ── Button-like elements ──
			"button" => {
				self.push_span(Span::styled("[", tui_style.style), cx.entity);
			}

			// ── Generic block handling ──
			_ => {
				// style already pushed by style_map.push above
			}
		}
	}

	fn leave_element(&mut self, cx: &VisitContext, element: &Element) {
		let name = element.tag();

		// Peek the style before popping so we can use lines_after.
		let tui_style = self.style_map.current();

		match name {
			// ── Headings ──
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				if tui_style.justify == Justify::Center {
					// Centered headings: render via render_text and map
					// the whole area to the heading entity since centering
					// shifts column offsets unpredictably.
					let span_entries = std::mem::take(&mut self.spans);
					if !span_entries.is_empty() {
						let (raw_spans, entities): (Vec<_>, Vec<_>) =
							span_entries.into_iter().unzip();
						let line = Line::from(raw_spans);
						let text = Text::from(line).style(tui_style.style);
						let before_y = self.area.y;
						// Alignment must be set on the Paragraph, not
						// the Text, because Paragraph ignores
						// Text.alignment and uses its own field.
						let paragraph = widgets::Paragraph::new(text)
							.wrap(widgets::Wrap { trim: false })
							.centered();
						let line_count =
							paragraph.line_count(self.area.width) as u16;
						paragraph.render(self.area, &mut self.buf);
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
			}

			// ── Paragraph ──
			"p" | "div" | "section" | "article" | "nav" | "header"
			| "footer" | "main" => {
				self.flush_spans();
			}

			// ── Blockquote ──
			"blockquote" | "aside" => {
				self.flush_spans();
				let indent = self.block_quote_indent;
				self.area.x = self.area.x.saturating_sub(indent);
				self.area.width = self.area.width.saturating_add(indent);
			}

			// ── Code blocks ──
			"pre" => {
				self.flush_spans();
				self.in_code_block = false;
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
			}
			"tbody" => {}
			"tr" => {
				self.flush_spans();
			}
			"th" | "td" => {
				// Cells are collected as spans; the row leave flushes them
			}

			// ── Links ──
			"a" => {}

			// ── Void elements, fully handled in visit_element ──
			"img" | "hr" | "br" => {}

			// ── Button ──
			"button" => {
				self.push_span(Span::styled("]", tui_style.style), cx.entity);
			}

			// ── Inline formatting ──
			"strong" | "b" | "em" | "i" | "del" | "s" | "code" => {
				// Style was applied per-span in visit_value
			}

			// ── Generic block handling ──
			_ => {
				if self.is_block_element(name) {
					self.flush_spans();
				}
			}
		}

		// Apply lines_after from TuiStyle for all elements.
		self.advance_lines(tui_style.lines_after);

		// Every push in visit_element is balanced by a pop here.
		self.style_map.pop();
	}

	fn visit_value(&mut self, cx: &VisitContext, value: &Value) {
		let text = value.to_string();
		if text.is_empty() {
			return;
		}

		// Build style from the style map's current nesting.
		let style = self.current_style();

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


#[cfg(test)]
mod test {
	use super::*;
	use ratatui::style::Modifier;

	/// System that renders an entity tree into a ratatui [`Buffer`]
	/// and returns both the buffer and span map.
	fn tui_render_system(
		In((entity, area)): In<(Entity, Rect)>,
		walker: NodeWalker,
	) -> (Buffer, TuiSpanMap) {
		let mut renderer = TuiNodeRenderer::new(area);
		walker.walk(&mut renderer, entity);
		renderer.finish()
	}

	/// Render a ratatui [`Buffer`] to a plain-text string, stripping
	/// trailing whitespace from each row.
	///
	/// Useful in tests to inspect rendered content without ANSI codes.
	fn buffer_to_text(buf: &Buffer) -> String {
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

	/// Helper: spawn an entity tree, walk it with [`TuiRenderer`],
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
		let bytes = MediaBytes::new_markdown(md);
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
		let bytes = MediaBytes::new_markdown("**bold**");
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

	#[test]
	fn custom_style_map() {
		let mut world = World::new();
		let text_entity = world.spawn(Value::from("Styled")).id();
		let heading =
			world.spawn((Element::new("h1"), Children::default())).id();
		world.entity_mut(heading).add_children(&[text_entity]);

		let area = Rect::new(0, 0, 40, 10);
		let custom_map = StyleMap::new(TuiStyle::default(), vec![(
			"h1",
			TuiStyle::new(Style::new().fg(Color::Red).bold()),
		)]);

		let buf = world
			.run_system_once_with(
				|In((entity, area, style_map)): In<(
					Entity,
					Rect,
					StyleMap<TuiStyle>,
				)>,
				 walker: NodeWalker| {
					let mut renderer =
						TuiNodeRenderer::new(area).with_style_map(style_map);
					walker.walk(&mut renderer, entity);
					let (buf, _tui_area) = renderer.finish();
					buf
				},
				(heading, area, custom_map),
			)
			.unwrap();

		buffer_to_text(&buf).as_str().xpect_contains("Styled");
		// Verify the custom red foreground was applied
		let buf_area = buf.area;
		let mut found_red = false;
		for row in buf_area.y..buf_area.y + buf_area.height {
			for col in buf_area.x..buf_area.x + buf_area.width {
				if buf[(col, row)].symbol() == "S"
					&& buf[(col, row)].style().fg == Some(Color::Red)
				{
					found_red = true;
				}
			}
		}
		found_red.xpect_true();
	}

	/// Verify h1 text is centered horizontally.
	#[test]
	fn h1_is_centered() {
		let mut world = World::new();
		let text_entity = world.spawn(Value::from("Hello")).id();
		let heading =
			world.spawn((Element::new("h1"), Children::default())).id();
		world.entity_mut(heading).add_children(&[text_entity]);

		let width = 40u16;
		let buf = render_to_buffer(&mut world, heading, width, 10);

		// Find the column where 'H' appears on the first non-empty row.
		let area = buf.area;
		let mut h_col: Option<u16> = None;
		'outer: for row in area.y..area.y + area.height {
			for col in area.x..area.x + area.width {
				if buf[(col, row)].symbol() == "H" {
					h_col = Some(col);
					break 'outer;
				}
			}
		}
		let col = h_col.unwrap();
		// Ratatui centers with: (area_width / 2) - (text_width / 2)
		let text_width = 5u16;
		let expected = (width / 2).saturating_sub(text_width / 2);
		col.xpect_eq(expected);
	}

	// -- TuiScrollState --

	#[test]
	fn scroll_state_no_overflow() {
		let state = TuiScrollState {
			offset: 0,
			content_height: 10,
			viewport_height: 20,
		};
		state.overflows().xpect_false();
		state.max_offset().xpect_eq(0);
	}

	#[test]
	fn scroll_state_overflow() {
		let state = TuiScrollState {
			offset: 0,
			content_height: 40,
			viewport_height: 20,
		};
		state.overflows().xpect_true();
		state.max_offset().xpect_eq(20);
	}

	#[test]
	fn scroll_down_clamps_to_max() {
		let mut state = TuiScrollState {
			offset: 0,
			content_height: 30,
			viewport_height: 20,
		};
		state.scroll_down(5);
		state.offset.xpect_eq(5);
		// Scrolling past max should clamp.
		state.scroll_down(100);
		state.offset.xpect_eq(10);
	}

	#[test]
	fn scroll_up_clamps_to_zero() {
		let mut state = TuiScrollState {
			offset: 3,
			content_height: 30,
			viewport_height: 20,
		};
		state.scroll_up(2);
		state.offset.xpect_eq(1);
		// Scrolling past zero should clamp.
		state.scroll_up(100);
		state.offset.xpect_eq(0);
	}

	#[test]
	fn clamp_adjusts_after_content_shrink() {
		let mut state = TuiScrollState {
			offset: 15,
			content_height: 40,
			viewport_height: 20,
		};
		// Simulate content shrinking so offset is now past max.
		state.content_height = 25;
		state.clamp();
		state.offset.xpect_eq(5);
	}
}
