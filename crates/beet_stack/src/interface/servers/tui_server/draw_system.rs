//! TUI rendering by attaching a Widget to select parts of a Card tree.
//!
//! [`RenderTui`] is the central system parameter that walks the card tree,
//! builds styled [`Line`] content from semantic components, and renders
//! into a scrollable view wrapped in a [`Block`].
//!
//! Diffing is handled by [`sync_tui_widgets`] which rebuilds
//! [`TuiWidget`] whenever [`TextContent`] changes on structural elements.
use super::widgets::ScrollableArea;
use crate::prelude::*;
use beet_core::prelude::*;
use ratatui::buffer::Buffer as RatBuffer;
use ratatui::prelude::Rect as RatRect;
use ratatui::prelude::Stylize;
use ratatui::prelude::Widget;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets;
use ratatui::widgets::WidgetRef;


// ---------------------------------------------------------------------------
// OwnedWidget wrapper
// ---------------------------------------------------------------------------

/// Wrapper that makes any `Widget + Clone` type usable as a [`WidgetRef`]
/// by cloning on each render. This bridges the gap between ratatui's
/// consuming `Widget::render(self, ..)` and the by-reference
/// `WidgetRef::render_ref(&self, ..)`.
struct OwnedWidget<W>(W);

impl<W: Widget + Clone> WidgetRef for OwnedWidget<W> {
	fn render_ref(&self, area: RatRect, buf: &mut RatBuffer) {
		self.0.clone().render(area, buf);
	}
}

// SAFETY: We only store Send + Sync inner widgets.
unsafe impl<W: Send> Send for OwnedWidget<W> {}
unsafe impl<W: Sync> Sync for OwnedWidget<W> {}


// ---------------------------------------------------------------------------
// TuiWidget component
// ---------------------------------------------------------------------------

/// Rendering binding attached to content entities.
///
/// Stores a boxed [`WidgetRef`] so any ratatui widget type can be
/// used, not just [`widgets::Paragraph`].
#[derive(Component)]
pub struct TuiWidget(pub Box<dyn WidgetRef + Send + Sync>);

impl TuiWidget {
	/// Create a new [`TuiWidget`] from any `Widget + Clone + Send + Sync`.
	///
	/// Wraps the widget in an [`OwnedWidget`] adapter so it can be
	/// rendered by reference via [`WidgetRef`].
	pub fn new(widget: impl Widget + Clone + Send + Sync + 'static) -> Self {
		Self(Box::new(OwnedWidget(widget)))
	}
}


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
	headings: Query<'w, 's, &'static Heading>,
	important: Query<'w, 's, (), With<Important>>,
	emphasize: Query<'w, 's, (), With<Emphasize>>,
	code: Query<'w, 's, (), With<Code>>,
	links: Query<'w, 's, &'static Link>,
}

impl RenderTui<'_, '_> {
	/// Build a [`TuiWidget`] for a [`Heading`] entity.
	fn build_heading(&self, entity: Entity) -> TuiWidget {
		let level = self
			.headings
			.get(entity)
			.map(|heading| heading.level())
			.unwrap_or(1);
		let spans = self.collect_styled_spans(entity);
		let line = if spans.is_empty() {
			Line::default()
		} else {
			Line::from(spans).bold()
		};
		// Level-1 headings are centered, deeper headings left-aligned
		let line = if level == 1 { line.centered() } else { line };
		TuiWidget::new(widgets::Paragraph::new(line))
	}

	/// Build a [`TuiWidget`] for a [`Paragraph`] entity.
	fn build_paragraph(&self, entity: Entity) -> TuiWidget {
		let spans = self.collect_styled_spans(entity);
		TuiWidget::new(
			widgets::Paragraph::new(Line::from(spans))
				.wrap(widgets::Wrap { trim: true }),
		)
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
// System: sync TuiWidget when text content changes
// ---------------------------------------------------------------------------

/// Rebuilds [`TuiWidget`] for [`DisplayBlock`] elements when their
/// text content changes. This is the single diffing location for
/// TUI widget updates.
pub(super) fn sync_tui_widgets(
	render: RenderTui,
	changed_text: Query<&ChildOf, Changed<TextContent>>,
	display_blocks: Query<Entity, With<DisplayBlock>>,
	headings: Query<(), With<Heading>>,
	children_query: Query<&Children>,
	text_content: Query<(), With<TextContent>>,
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

	// Also rebuild display blocks that have text children but no
	// TuiWidget yet (first run)
	for entity in &display_blocks {
		if !dirty.contains(&entity) {
			if let Ok(children) = children_query.get(entity) {
				for child in children.iter() {
					if text_content.contains(child) {
						dirty.push(entity);
						break;
					}
				}
			}
		}
	}

	for entity in dirty {
		let widget = if headings.contains(entity) {
			render.build_heading(entity)
		} else {
			render.build_paragraph(entity)
		};
		commands.entity(entity).insert(widget);
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
// Collect renderable content into a composed Text widget
// ---------------------------------------------------------------------------

/// Collects [`TuiWidget`] entities and tool buttons into a single
/// [`Text`] block for rendering.
fn collect_content_text(
	card_entity: Entity,
	card_query: &CardQuery,
	text_query: &TextQuery,
	widget_query: &Query<&TuiWidget>,
	tool_query: &Query<&ToolMeta>,
) -> (String, Text<'static>) {
	let mut block_title = String::new();
	let mut lines: Vec<Line<'static>> = Vec::new();

	// Use main_heading helper to extract block title
	if let Some((_heading_entity, title_text)) =
		text_query.main_heading(card_entity)
	{
		block_title = format!(" {} ", title_text);
	}

	// Render a temporary buffer per widget to extract lines
	for entity in card_query.iter_dfs(card_entity) {
		if let Ok(tui_widget) = widget_query.get(entity) {
			let temp_area = RatRect::new(0, 0, 200, 50);
			let mut temp_buf = RatBuffer::empty(temp_area);
			tui_widget.0.render_ref(temp_area, &mut temp_buf);

			// Read lines from the buffer
			for row in 0..temp_area.height {
				let mut line_str = String::new();
				for col in 0..temp_area.width {
					let cell = &temp_buf[(col, row)];
					line_str.push_str(cell.symbol());
				}
				let trimmed = line_str.trim_end().to_string();
				if !trimmed.is_empty() {
					lines.push(Line::from(trimmed));
				}
			}
		}
	}

	// Collect tool buttons using the handler name from ToolMeta
	let mut buttons: Vec<String> = Vec::new();
	for entity in card_query.iter_dfs(card_entity) {
		if let Ok(meta) = tool_query.get(entity) {
			if meta.input().type_id() == std::any::TypeId::of::<()>() {
				// Extract the short name from the full type path
				let full_name = meta.name();
				let short_name =
					full_name.rsplit("::").next().unwrap_or(full_name);
				buttons.push(short_name.to_string());
			}
		}
	}

	if !buttons.is_empty() {
		lines.push(Line::default());
		for label in &buttons {
			lines.push(Line::from(format!("[ {} ]", label)).centered());
		}
	}

	(block_title, Text::from(lines))
}


// ---------------------------------------------------------------------------
// Draw system
// ---------------------------------------------------------------------------

/// Renders the current card's content tree into the terminal each frame.
///
/// Collects widgets via [`CardQuery`], wraps them in a [`Block`] with
/// the card's main heading, and renders with scrollbar support via
/// [`ScrollableArea`].
pub(super) fn draw_system(
	mut context: ResMut<bevy_ratatui::RatatuiContext>,
	card: Query<(Entity, Option<&TuiScrollState>), With<CurrentCard>>,
	card_query: CardQuery,
	text_query: TextQuery,
	widget_query: Query<&TuiWidget>,
	tool_query: Query<&ToolMeta>,
) -> Result {
	let (card_entity, scroll_state) = card.single()?;
	let scroll_offset = scroll_state.map(|ss| ss.offset).unwrap_or(0);

	context.draw(|frame| {
		let area = frame.area();

		let (block_title, content_text) = collect_content_text(
			card_entity,
			&card_query,
			&text_query,
			&widget_query,
			&tool_query,
		);

		let block = widgets::Block::bordered()
			.title(Line::from(block_title).bold().centered());
		let inner_area = block.inner(area);
		frame.render_widget(block, area);

		if content_text.lines.is_empty() {
			return;
		}

		let total_lines = content_text.lines.len();

		let paragraph = widgets::Paragraph::new(content_text)
			.wrap(widgets::Wrap { trim: true })
			.scroll((scroll_offset, 0));

		let scrollable =
			ScrollableArea::new(paragraph, total_lines, scroll_offset as usize);
		frame.render_widget(scrollable, inner_area);
	})?;
	Ok(())
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
