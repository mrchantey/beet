//! TUI rendering by attaching a Widget to select parts of a Card tree.
//!
//! [`RenderTui`] is the central system parameter that walks the card tree,
//! builds styled [`Line`] content from semantic components, and renders
//! into a scrollable [`ratatui::widgets::Paragraph`] wrapped in a [`Block`].
//!
//! Diffing is handled by [`sync_tui_widgets`] which rebuilds
//! [`TuiWidget`] whenever [`TextContent`] changes on structural elements.
use crate::prelude::*;
use beet_core::prelude::*;
use ratatui::buffer::Buffer as RatBuffer;
use ratatui::prelude::Constraint;
use ratatui::prelude::Layout as RatLayout;
use ratatui::prelude::Rect as RatRect;
use ratatui::prelude::Stylize;
use ratatui::prelude::Widget;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;


// ---------------------------------------------------------------------------
// TuiWidget component
// ---------------------------------------------------------------------------

/// Rendering binding attached to content entities.
///
/// Stores a ratatui [`widgets::Paragraph`] with owned (`'static`) data
/// so it can live as a Bevy component without lifetime issues.
#[derive(Component)]
pub struct TuiWidget(pub widgets::Paragraph<'static>);


// ---------------------------------------------------------------------------
// Scroll state
// ---------------------------------------------------------------------------

/// Per-card scroll position, inserted alongside [`CurrentCard`].
#[derive(Default, Component)]
pub struct TuiScrollState {
	/// Current vertical scroll offset (in lines).
	pub offset: u16,
}


// ---------------------------------------------------------------------------
// RenderTui SystemParam
// ---------------------------------------------------------------------------

/// System parameter for building TUI widgets from the semantic content tree.
///
/// Uses [`CardQuery`] for scoped traversal and [`TextQuery`] for
/// collecting text from structural elements.
#[derive(SystemParam)]
pub(super) struct RenderTui<'w, 's> {
	text_query: TextQuery<'w, 's>,
	text_content: Query<'w, 's, &'static TextContent>,
	important: Query<'w, 's, (), With<Important>>,
	emphasize: Query<'w, 's, (), With<Emphasize>>,
	code: Query<'w, 's, (), With<Code>>,
	links: Query<'w, 's, &'static Link>,
}

impl RenderTui<'_, '_> {
	/// Build a [`TuiWidget`] paragraph for a [`Title`] entity.
	fn build_title(&self, entity: Entity) -> widgets::Paragraph<'static> {
		let level = self.text_query.title_level(entity);
		let spans = self.collect_styled_spans(entity);
		let line = if spans.is_empty() {
			Line::default()
		} else {
			Line::from(spans).bold()
		};
		// Level 0 titles are centered, deeper titles left-aligned
		let line = if level == 0 { line.centered() } else { line };
		widgets::Paragraph::new(line)
	}

	/// Build a [`TuiWidget`] paragraph for a [`Paragraph`] entity.
	fn build_paragraph(&self, entity: Entity) -> widgets::Paragraph<'static> {
		let spans = self.collect_styled_spans(entity);
		widgets::Paragraph::new(Line::from(spans))
			.wrap(widgets::Wrap { trim: true })
	}

	/// Collect styled [`Span`] from the children of a structural element,
	/// applying inline markers.
	fn collect_styled_spans(&self, parent: Entity) -> Vec<Span<'static>> {
		let mut spans = Vec::new();
		let entities = self.text_query.collect_text_entities(parent);

		for child in entities {
			let Ok(text) = self.text_content.get(child) else {
				continue;
			};
			let mut span = Span::raw(text.as_str().to_owned());

			// Apply inline styles
			if self.important.contains(child) {
				span = span.bold();
			}
			if self.emphasize.contains(child) {
				span = span.italic();
			}
			if self.code.contains(child) {
				span = span.dim();
			}
			if self.links.contains(child) {
				span = span.underlined();
			}

			spans.push(span);
		}
		spans
	}
}


// ---------------------------------------------------------------------------
// Hyperlink widget
// ---------------------------------------------------------------------------

/// A terminal hyperlink using OSC 8 escape sequences.
///
/// Renders clickable links in supported terminals.
pub struct TuiHyperlink {
	text: Text<'static>,
	url: String,
}

impl TuiHyperlink {
	/// Create a hyperlink with the given display text and URL.
	pub fn new(text: impl Into<Text<'static>>, url: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			url: url.into(),
		}
	}
}

impl Widget for &TuiHyperlink {
	fn render(self, area: RatRect, buffer: &mut RatBuffer) {
		(&self.text).render(area, buffer);

		// OSC 8 hyperlink rendering in 2-char chunks to work around
		// ratatui's ANSI width calculation bug.
		let chars: Vec<char> = self.text.to_string().chars().collect();
		for (idx, chunk) in chars.chunks(2).enumerate() {
			let text: String = chunk.iter().collect();
			let hyperlink =
				format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, text);
			let col = area.x + idx as u16 * 2;
			if col < area.x + area.width {
				buffer[(col, area.y)].set_symbol(hyperlink.as_str());
			}
		}
	}
}


// ---------------------------------------------------------------------------
// Button widget
// ---------------------------------------------------------------------------

/// A simple TUI button for invoking tools with `()` input.
#[allow(dead_code)]
#[derive(Clone)]
pub struct TuiButton {
	label: Line<'static>,
	selected: bool,
}

#[allow(dead_code)]
impl TuiButton {
	/// Create a button with the given label.
	pub fn new(label: impl Into<Line<'static>>) -> Self {
		Self {
			label: label.into(),
			selected: false,
		}
	}

	/// Mark this button as selected.
	pub fn selected(mut self, selected: bool) -> Self {
		self.selected = selected;
		self
	}
}

impl Widget for TuiButton {
	fn render(self, area: RatRect, buf: &mut RatBuffer) {
		use ratatui::style::Color;
		use ratatui::style::Style;
		let (bg, fg) = if self.selected {
			(Color::White, Color::Black)
		} else {
			(Color::DarkGray, Color::White)
		};
		buf.set_style(area, Style::new().bg(bg).fg(fg));

		// Top highlight
		if area.height > 2 {
			buf.set_string(
				area.x,
				area.y,
				"\u{2594}".repeat(area.width as usize),
				Style::new().fg(Color::Gray).bg(bg),
			);
		}
		// Bottom shadow
		if area.height > 1 {
			buf.set_string(
				area.x,
				area.y + area.height - 1,
				"\u{2581}".repeat(area.width as usize),
				Style::new().fg(Color::Black).bg(bg),
			);
		}
		// Label centered
		buf.set_line(
			area.x + (area.width.saturating_sub(self.label.width() as u16)) / 2,
			area.y + (area.height.saturating_sub(1)) / 2,
			&self.label,
			area.width,
		);
	}
}


// ---------------------------------------------------------------------------
// System: sync TuiWidget when text content changes
// ---------------------------------------------------------------------------

/// Rebuilds [`TuiWidget`] for structural elements when their
/// text content changes. This is the single diffing location for
/// TUI widget updates.
pub(super) fn sync_tui_widgets(
	render: RenderTui,
	changed_text: Query<&ChildOf, Changed<TextContent>>,
	titles: Query<Entity, With<Title>>,
	paragraphs: Query<Entity, With<Paragraph>>,
	children_query: Query<&Children>,
	mut commands: Commands,
) {
	// Collect structural parents whose text children changed
	let mut dirty = Vec::new();
	for child_of in &changed_text {
		let parent = child_of.parent();
		if !dirty.contains(&parent) {
			dirty.push(parent);
		}
	}

	// Also rebuild all titles/paragraphs on first run (no changes yet)
	// by handling the case where entities are newly added
	for entity in &titles {
		if !dirty.contains(&entity) {
			// Check if any child has TextContent - if widget doesn't
			// exist yet, we should build it
			if let Ok(children) = children_query.get(entity) {
				for child in children.iter() {
					if render.text_content.contains(child) {
						dirty.push(entity);
						break;
					}
				}
			}
		}
	}
	for entity in &paragraphs {
		if !dirty.contains(&entity) {
			if let Ok(children) = children_query.get(entity) {
				for child in children.iter() {
					if render.text_content.contains(child) {
						dirty.push(entity);
						break;
					}
				}
			}
		}
	}

	for entity in dirty {
		let widget = if titles.contains(entity) {
			render.build_title(entity)
		} else if paragraphs.contains(entity) {
			render.build_paragraph(entity)
		} else {
			continue;
		};
		commands.entity(entity).insert(TuiWidget(widget));
	}
}


// ---------------------------------------------------------------------------
// System: ensure scroll state exists on CurrentCard
// ---------------------------------------------------------------------------

/// Inserts [`TuiScrollState`] on the current card if missing.
pub(super) fn ensure_scroll_state(
	card: Query<Entity, (With<CurrentCard>, Without<TuiScrollState>)>,
	mut commands: Commands,
) {
	for entity in &card {
		commands.entity(entity).insert(TuiScrollState::default());
	}
}


// ---------------------------------------------------------------------------
// Draw system
// ---------------------------------------------------------------------------

/// Renders the current card's content tree into the terminal each frame.
///
/// Collects widgets via [`CardQuery`], wraps them in a [`Block`] with
/// the card's title, and renders with scrollbar support.
pub(super) fn draw_system(
	mut context: ResMut<bevy_ratatui::RatatuiContext>,
	card: Query<(Entity, Option<&TuiScrollState>), With<CurrentCard>>,
	card_query: CardQuery,
	text_query: TextQuery,
	widget_query: Query<&TuiWidget>,
	title_widget_query: Query<&TuiWidget, With<Title>>,
	tool_query: Query<(&ToolMeta, &PathPartial)>,
) -> Result {
	let (card_entity, scroll_state) = card.single()?;
	let scroll_offset = scroll_state.map(|ss| ss.offset).unwrap_or(0);

	context.draw(|frame| {
		let area = frame.area();

		// Find the block title: first level-0 Title in the card
		let mut block_title = String::new();
		let mut renderables: Vec<Entity> = Vec::new();

		for entity in card_query.iter_dfs(card_entity) {
			if widget_query.contains(entity) {
				// Check if this is the root title (level 0) for block title
				if block_title.is_empty() {
					if text_query.is_title(entity)
						&& text_query.title_level(entity) == 0
					{
						if let Ok(title_widget) = title_widget_query.get(entity)
						{
							block_title = format!(
								" {} ",
								extract_paragraph_text(&title_widget.0)
							);
							continue;
						}
					}
				}
				renderables.push(entity);
			}
		}

		// Collect tool buttons for tools with () input
		let mut buttons: Vec<String> = Vec::new();
		for entity in card_query.iter_dfs(card_entity) {
			if let Ok((meta, path)) = tool_query.get(entity) {
				if meta.input().type_id() == std::any::TypeId::of::<()>() {
					let label = path
						.segments
						.iter()
						.map(|s| s.as_str())
						.collect::<Vec<_>>()
						.join("/");
					buttons.push(label);
				}
			}
		}

		// Build the block
		let block = widgets::Block::bordered()
			.title(Line::from(block_title).bold().centered());

		let inner_area = block.inner(area);
		frame.render_widget(block, area);

		if renderables.is_empty() && buttons.is_empty() {
			return;
		}

		// Calculate content height
		let mut content_lines: Vec<Line<'static>> = Vec::new();
		for entity in &renderables {
			if let Ok(tui_widget) = widget_query.get(*entity) {
				// Render widget into a temporary buffer to count lines
				let text = extract_paragraph_text(&tui_widget.0);
				if !text.is_empty() {
					content_lines.push(Line::from(text));
				} else {
					content_lines.push(Line::default());
				}
			}
		}

		// Add blank line before buttons
		if !buttons.is_empty() {
			content_lines.push(Line::default());
			for label in &buttons {
				content_lines
					.push(Line::from(format!("[ {} ]", label)).centered());
			}
		}

		let total_lines = content_lines.len();

		// Build a combined paragraph with scroll
		let paragraph = widgets::Paragraph::new(content_lines)
			.wrap(widgets::Wrap { trim: true })
			.scroll((scroll_offset, 0));

		// Only show scrollbar when content exceeds viewport
		let show_scrollbar = total_lines as u16 > inner_area.height;

		if show_scrollbar {
			// Split inner area: content | scrollbar
			let chunks = RatLayout::horizontal([
				Constraint::Min(1),
				Constraint::Length(1),
			])
			.split(inner_area);

			frame.render_widget(paragraph, chunks[0]);

			let mut scrollbar_state = ScrollbarState::new(total_lines)
				.position(scroll_offset as usize);
			frame.render_stateful_widget(
				Scrollbar::new(ScrollbarOrientation::VerticalRight)
					.begin_symbol(None)
					.end_symbol(None),
				chunks[1],
				&mut scrollbar_state,
			);
		} else {
			frame.render_widget(paragraph, inner_area);
		}
	})?;
	Ok(())
}

/// Extract the raw text content from a [`widgets::Paragraph`].
///
/// This is a workaround since ratatui paragraphs don't expose their
/// text directly. We format via Debug and extract what we can.
fn extract_paragraph_text(para: &widgets::Paragraph<'static>) -> String {
	// Use the paragraph's internal text - we need to render it to extract
	// Since Paragraph doesn't expose its Text, we'll render to a buffer
	let test_area = RatRect::new(0, 0, 200, 100);
	let mut buf = RatBuffer::empty(test_area);
	Widget::render(para.clone(), test_area, &mut buf);

	// Read back from the buffer
	let mut lines = Vec::new();
	for row in 0..test_area.height {
		let mut line = String::new();
		for col in 0..test_area.width {
			let cell = &buf[(col, row)];
			line.push_str(cell.symbol());
		}
		let trimmed = line.trim_end().to_string();
		if !trimmed.is_empty() || !lines.is_empty() {
			lines.push(trimmed);
		}
	}

	// Remove trailing empty lines
	while lines.last().is_some_and(|line| line.is_empty()) {
		lines.pop();
	}

	lines.join("")
}


// ---------------------------------------------------------------------------
// Input: scroll handling
// ---------------------------------------------------------------------------

/// Handle scroll input events for the TUI.
pub(super) fn handle_scroll_input(
	mut messages: MessageReader<bevy::input::keyboard::KeyboardInput>,
	mut scroll_query: Query<&mut TuiScrollState, With<CurrentCard>>,
) {
	use bevy::input::keyboard::Key;
	for message in messages.read() {
		let Ok(mut scroll) = scroll_query.single_mut() else {
			return;
		};
		match &message.logical_key {
			Key::Character(val) if val == "j" => {
				scroll.offset = scroll.offset.saturating_add(1);
			}
			Key::Character(val) if val == "k" => {
				scroll.offset = scroll.offset.saturating_sub(1);
			}
			Key::ArrowDown => {
				scroll.offset = scroll.offset.saturating_add(1);
			}
			Key::ArrowUp => {
				scroll.offset = scroll.offset.saturating_sub(1);
			}
			_ => {}
		}
	}
}
