//! Composable TUI rendering via the [`Renderable`] trait.
//!
//! Each semantic content type ([`Heading`], [`Paragraph`]) implements
//! [`Renderable`] to check if an entity has its component and render
//! it directly to the ratatui buffer. Multiple types compose via
//! tuples, analogous to [`TokenizeComponents`](beet_core::prelude::TokenizeComponents).
//!
//! The [`draw_system`] is an exclusive system that walks the card
//! tree via DFS and calls [`Renderable::render`] for each entity.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy_ratatui::RatatuiContext;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
use ratatui::style::Stylize;
use ratatui::widgets;
use ratatui::widgets::Widget;
use variadics_please::all_tuples;


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
// Draw system
// ---------------------------------------------------------------------------

/// Renders the current card's content tree into the terminal each frame.
///
/// Walks the card tree via DFS and calls [`Renderable::render`] for each
/// entity, composing multiple renderable types via the `D` type parameter.
pub(super) fn draw_system<D: Renderable>(world: &mut World) -> Result {
	let card_entity = world
		.query_filtered::<Entity, With<CurrentCard>>()
		.single(world)?;

	let entities = card_dfs(world, card_entity);

	world.resource_scope(
		|world: &mut World, mut context: Mut<RatatuiContext>| {
			bevy_draw(&mut context, |frame| {
				let mut area = frame.area();
				let buffer = frame.buffer_mut();
				for &entity in &entities {
					D::render(&mut area, buffer, world, entity)?;
				}
				Ok(())
			})
		},
	)
}

/// [`BevyError`] compatible version of [`RatatuiContext::draw`].
fn bevy_draw(
	cx: &mut RatatuiContext,
	func: impl FnOnce(&mut Frame) -> Result,
) -> Result {
	let mut result = Ok(());
	cx.draw(|frame| {
		result = func(frame);
	})?;
	result
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


// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// DFS traversal within card boundaries, stopping at nested [`Card`] entities.
fn card_dfs(world: &World, root: Entity) -> Vec<Entity> {
	let mut result = Vec::new();
	let mut stack = vec![root];
	while let Some(entity) = stack.pop() {
		result.push(entity);
		if let Some(children) = world.entity(entity).get::<Children>() {
			for child in children.iter().rev() {
				// Stop at Card boundaries, but not the root itself
				if child != root && world.entity(child).contains::<Card>() {
					continue;
				}
				stack.push(child);
			}
		}
	}
	result
}

/// Collects plain text from a structural element's direct children,
/// skipping nested structural elements.
fn collect_text(world: &World, parent: Entity) -> String {
	let Some(children) = world.entity(parent).get::<Children>() else {
		return String::new();
	};
	let mut result = String::new();
	for child in children.iter() {
		let child_ref = world.entity(child);
		if child_ref.contains::<DisplayBlock>() {
			continue;
		}
		if let Some(text) = child_ref.get::<TextContent>() {
			result.push_str(text.as_str());
		}
	}
	result
}


// ---------------------------------------------------------------------------
// Renderable trait
// ---------------------------------------------------------------------------

/// Trait for rendering content components to the terminal.
///
/// Each component type implements this to check if an entity has its
/// component and render it directly to the ratatui buffer. Compose
/// multiple types via tuples, analogous to
/// [`TokenizeComponents`](beet_core::prelude::TokenizeComponents).
pub(super) trait Renderable: 'static + Send + Sync {
	/// Render this component on the entity if present.
	fn render(
		area: &mut Rect,
		buf: &mut Buffer,
		world: &World,
		entity: Entity,
	) -> Result;
}


// ---------------------------------------------------------------------------
// Renderable implementations
// ---------------------------------------------------------------------------

impl Renderable for Heading {
	fn render(
		area: &mut Rect,
		buf: &mut Buffer,
		world: &World,
		entity: Entity,
	) -> Result {
		let Some(heading) = world.entity(entity).get::<Heading>() else {
			return Ok(());
		};
		let text = collect_text(world, entity);
		let line = ratatui::prelude::Line::from(text).bold();
		let _level = heading.level();
		render_text_consuming(area, buf, line);
		Ok(())
	}
}

fn render_text_consuming<'a>(
	area: &mut Rect,
	buf: &mut Buffer,
	text: impl Into<ratatui::prelude::Text<'a>>,
) {
	let text = text.into();
	// return if empty
	if text.iter().len() == 0 {
		return;
	}
	let paragraph =
		widgets::Paragraph::new(text).wrap(widgets::Wrap { trim: false });
	let len = paragraph.line_count(area.width);
	// 1. render
	paragraph.render(*area, buf);
	// 2. consume lines
	area.y = area.y.saturating_add(len as u16);
	area.height = area.height.saturating_sub(len as u16);
}

impl Renderable for Paragraph {
	fn render(
		area: &mut Rect,
		buf: &mut Buffer,
		world: &World,
		entity: Entity,
	) -> Result {
		if !world.entity(entity).contains::<Paragraph>() {
			return Ok(());
		}
		let text = collect_text(world, entity);
		render_text_consuming(area, buf, text);
		Ok(())
	}
}


// ---------------------------------------------------------------------------
// Tuple composition
// ---------------------------------------------------------------------------

macro_rules! impl_renderable_tuple {
	($(($T:ident, $t:ident)),*) => {
		impl<$($T: Renderable),*> Renderable for ($($T,)*) {
			fn render(
				area: &mut Rect,
				buf: &mut Buffer,
				world: &World,
				entity: Entity,
			) -> Result {
				$(<$T as Renderable>::render(area, buf, world, entity)?;)*
				Ok(())
			}
		}
	}
}

all_tuples!(impl_renderable_tuple, 1, 15, T, t);


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn heading_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Heading1, children![(TextContent::new("Hello World"),)]))
			.id();

		let mut area = Rect::new(0, 0, 80, 24);
		let mut buf = Buffer::empty(area);

		Heading::render(&mut area, &mut buf, &world, entity).unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("Hello World");
	}

	#[test]
	fn paragraph_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Paragraph, children![(TextContent::new("body text"),)]))
			.id();

		let mut area = Rect::new(0, 0, 80, 24);
		let mut buf = Buffer::empty(area);

		Paragraph::render(&mut area, &mut buf, &world, entity).unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("body text");
	}

	#[test]
	fn tuple_renderable_calls_all() {
		let mut world = World::new();
		let entity = world
			.spawn((Heading1, children![(TextContent::new("Title"),)]))
			.id();

		let mut area = Rect::new(0, 0, 80, 24);
		let mut buf = Buffer::empty(area);

		<(Heading,)>::render(&mut area, &mut buf, &world, entity).unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("Title");
	}

	#[test]
	fn skips_entity_without_component() {
		let mut world = World::new();
		let entity = world.spawn(TextContent::new("not a heading")).id();

		let mut area = Rect::new(0, 0, 80, 24);
		let mut buf = Buffer::empty(area);

		// Should not panic, just skip
		<(Heading, Paragraph)>::render(&mut area, &mut buf, &world, entity)
			.unwrap();
	}
}
