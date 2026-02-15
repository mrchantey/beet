//! TUI rendering via typed [`TuiWidget<T>`] components.
//!
//! Each semantic content type gets its own concrete `TuiWidget<W>` that
//! is inserted/removed via generic lifecycle observers and rebuilt when
//! the underlying [`TextContent`] changes.
//!
//! The [`draw_system`] walks the card tree via [`CardQuery`] DFS and
//! composes [`Line`] from child [`TuiWidget<Span>`] entities, rendering
//! structural elements (headings, paragraphs) directly without
//! intermediate buffers.
use super::widgets::Hyperlink;
use crate::prelude::*;
use beet_core::prelude::*;
use ratatui::prelude::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text as RatText;
use ratatui::widgets;


// ---------------------------------------------------------------------------
// TuiWidget<T> component
// ---------------------------------------------------------------------------

/// Typed rendering binding attached to content entities.
///
/// Each ratatui widget type gets its own ECS component, enabling
/// independent queries and targeted rebuilds.
#[derive(Component)]
pub struct TuiWidget<T: Send + Sync + 'static>(pub T);

impl<T: Send + Sync + 'static> TuiWidget<T> {
	/// Wrap a widget value in a [`TuiWidget`].
	pub fn new(widget: T) -> Self { Self(widget) }
}

impl<T: Default + Send + Sync + 'static> Default for TuiWidget<T> {
	fn default() -> Self { Self(T::default()) }
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
// Generic widget lifecycle
// ---------------------------------------------------------------------------

/// Bind the lifecycle of `TuiWidget<W>` to semantic component `S`.
///
/// Inserts a default `TuiWidget<W>` when `S` is added to an entity
/// and removes it when `S` is removed.
pub(super) fn widget_lifecycle_plugin<
	S: Component,
	W: Default + Send + Sync + 'static,
>(
	app: &mut App,
) {
	app.add_observer(insert_widget::<S, W>)
		.add_observer(remove_widget::<S, W>);
}

/// Observer: insert a default [`TuiWidget<W>`] when `S` is added.
fn insert_widget<S: Component, W: Default + Send + Sync + 'static>(
	ev: On<Insert, S>,
	mut commands: Commands,
) {
	commands.entity(ev.entity).insert(TuiWidget::<W>::default());
}

/// Observer: remove [`TuiWidget<W>`] when `S` is removed.
fn remove_widget<S: Component, W: Default + Send + Sync + 'static>(
	ev: On<Remove, S>,
	mut commands: Commands,
) {
	commands.entity(ev.entity).remove::<TuiWidget<W>>();
}


// ---------------------------------------------------------------------------
// Span rebuild
// ---------------------------------------------------------------------------

/// Rebuild individual span widgets when their [`TextContent`] changes,
/// applying inline semantic markers as ratatui styles.
pub(super) fn rebuild_tui_span(
	query: Query<(Entity, &TextContent), Changed<TextContent>>,
	important: Query<(), With<Important>>,
	emphasize: Query<(), With<Emphasize>>,
	code: Query<(), With<Code>>,
	links: Query<(), With<Link>>,
	mut commands: Commands,
) {
	for (entity, text) in &query {
		let mut span = Span::raw(text.as_str().to_owned());

		if important.contains(entity) {
			span = span.bold();
		}
		if emphasize.contains(entity) {
			span = span.italic();
		}
		if code.contains(entity) {
			span = span.dim();
		}
		if links.contains(entity) {
			span = span.underlined();
		}

		commands.entity(entity).insert(TuiWidget::new(span));
	}
}


// ---------------------------------------------------------------------------
// Hyperlink rebuild
// ---------------------------------------------------------------------------

/// Rebuild hyperlink widgets when [`TextContent`] or [`Link`] changes.
pub(super) fn rebuild_tui_hyperlink(
	query: Query<
		(Entity, &TextContent, &Link),
		Or<(Changed<TextContent>, Changed<Link>)>,
	>,
	mut commands: Commands,
) {
	for (entity, text, link) in &query {
		let widget =
			Hyperlink::new(text.as_str().to_owned(), link.href.clone());
		commands.entity(entity).insert(TuiWidget::new(widget));
	}
}


// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Collect child [`TuiWidget<Span>`] values from a structural element's
/// direct children, preserving child order.
fn collect_child_spans(
	parent: Entity,
	children_query: &Query<&Children>,
	spans: &Query<&TuiWidget<Span<'static>>>,
) -> Vec<Span<'static>> {
	let Ok(children) = children_query.get(parent) else {
		return Vec::new();
	};
	children
		.iter()
		.filter_map(|child| spans.get(child).ok())
		.map(|widget| widget.0.clone())
		.collect()
}


// ---------------------------------------------------------------------------
// Draw system
// ---------------------------------------------------------------------------

/// Renders the current card's content tree into the terminal each frame.
///
/// Walks the card tree via [`CardQuery`] DFS. For each structural
/// element ([`Heading`], [`Paragraph`]), collects child
/// [`TuiWidget<Span>`] values and composes them into [`Line`].
/// The resulting lines are rendered as a single [`widgets::Paragraph`]
/// inside a bordered block titled with the card's main heading.
pub(super) fn draw_system(
	mut context: ResMut<bevy_ratatui::RatatuiContext>,
	card: Query<Entity, With<CurrentCard>>,
	card_query: CardQuery,
	text_query: TextQuery,
	headings: Query<&Heading>,
	paragraphs: Query<(), With<Paragraph>>,
	spans: Query<&TuiWidget<Span<'static>>>,
	children_query: Query<&Children>,
) -> Result {
	let card_entity = card.single()?;

	context.draw(|frame| {
		let area = frame.area();

		let block_title = text_query
			.main_heading(card_entity)
			.map(|(_, title)| format!(" {} ", title))
			.unwrap_or_default();

		let block = widgets::Block::bordered()
			.title(Line::from(block_title).bold().centered());
		let inner_area = block.inner(area);
		frame.render_widget(block, area);

		let mut lines: Vec<Line<'static>> = Vec::new();

		for entity in card_query.iter_dfs(card_entity) {
			if let Ok(heading) = headings.get(entity) {
				let child_spans =
					collect_child_spans(entity, &children_query, &spans);
				if child_spans.is_empty() {
					continue;
				}
				let line = Line::from(child_spans).bold();
				lines.push(if heading.level() == 1 {
					line.centered()
				} else {
					line
				});
			} else if paragraphs.contains(entity) {
				let child_spans =
					collect_child_spans(entity, &children_query, &spans);
				if !child_spans.is_empty() {
					lines.push(Line::from(child_spans));
				}
			}
		}

		if lines.is_empty() {
			return;
		}

		let paragraph = widgets::Paragraph::new(RatText::from(lines))
			.wrap(widgets::Wrap { trim: true });
		frame.render_widget(paragraph, inner_area);
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
