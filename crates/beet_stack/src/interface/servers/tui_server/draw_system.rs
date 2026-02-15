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
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets;
use ratatui::widgets::Widget;
use variadics_please::all_tuples;

/// Renders the current card's content tree into the terminal each frame.
///
/// Walks the card tree via DFS and calls [`Renderable::render`] for each
/// entity, composing multiple renderable types via the `D` type parameter.
pub(super) fn draw_system<D: Renderable>(world: &mut World) -> Result {
	let card_entity = world
		.query_filtered::<Entity, With<CurrentCard>>()
		.single(world)?;

	let entities = CardQuery::iter_dfs_exclusive(world, card_entity);

	world.resource_scope(
		|world: &mut World, mut context: Mut<RatatuiContext>| {
			bevy_draw(&mut context, |frame| {
				let mut area = frame.area();
				let buffer = frame.buffer_mut();
				for &entity in &entities {
					let mut entity_mut = world.entity_mut(entity);
					D::render(&mut area, buffer, &mut entity_mut)?;
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

/// Collects plain text from a structural element's direct children,
/// skipping nested structural elements.
fn collect_text<'a>(world: &'a World, parent: Entity) -> Text<'a> {
	let Some(children) = world.entity(parent).get::<Children>() else {
		return default();
	};
	let mut spans = Vec::new();
	for child in children.iter() {
		let child_ref = world.entity(child);
		if child_ref.contains::<DisplayBlock>() {
			continue;
		}
		if let Some(text) = child_ref.get::<TextNode>() {
			let span =
				Span::from(text.as_str()).style(styled_entity(child_ref));
			spans.push(span);
		}
	}
	let line: Line<'a> = spans.into();
	// we use Text for its wrapping capabilities
	Text::from(line).style(styled_entity(world.entity(parent)))
}

fn styled_entity(entity: EntityRef) -> Style {
	let mut style = Style::new();

	if entity.contains::<Emphasize>() {
		style = style.italic();
	};
	if entity.contains::<Important>() {
		style = style.bold();
	}
	if entity.contains::<Code>() {
		// TODO design tokens
		style = style.bg(ratatui::style::Color::DarkGray);
	}
	style
}

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
		entity: &mut EntityWorldMut,
	) -> Result;
}

impl Renderable for Heading {
	fn render(
		area: &mut Rect,
		buf: &mut Buffer,
		entity: &mut EntityWorldMut,
	) -> Result {
		let Some(heading) = entity.get::<Heading>() else {
			return Ok(());
		};
		let world = entity.world();
		let entity_id = entity.id();
		let mut text = collect_text(world, entity_id).bold();
		// let line = ratatui::prelude::Line::from(text).bold();
		// TODO use design tokens, ie primary color, to render heading level
		let level = heading.level();
		if level == 1 {
			// h1: centered with gap above
			text = text.centered();
			area.y = area.y.saturating_add(1);
		} else {
			// h2-6: indent based on level
			text.lines.first_mut().map(|line| {
				line.spans.insert(0, Span::raw(" ".repeat(level as usize)));
				line
			});
		}
		render_text_consuming(area, buf, text);
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
		entity: &mut EntityWorldMut,
	) -> Result {
		if !entity.contains::<Paragraph>() {
			return Ok(());
		}
		let world = entity.world();
		let entity_id = entity.id();
		let text = collect_text(world, entity_id);
		render_text_consuming(area, buf, text);
		Ok(())
	}
}

macro_rules! impl_renderable_tuple {
	($(($T:ident, $t:ident)),*) => {
		impl<$($T: Renderable),*> Renderable for ($($T,)*) {
			fn render(
				area: &mut Rect,
				buf: &mut Buffer,
				entity: &mut EntityWorldMut,
			) -> Result {
				$(<$T as Renderable>::render(area, buf, entity)?;)*
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
			.spawn((Heading1, children![(TextNode::new("Hello World"),)]))
			.id();

		let mut area = Rect::new(0, 0, 80, 24);
		let mut buf = Buffer::empty(area);

		let mut entity_mut = world.entity_mut(entity);
		Heading::render(&mut area, &mut buf, &mut entity_mut).unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("Hello World");
	}

	#[test]
	fn paragraph_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((Paragraph, children![(TextNode::new("body text"),)]))
			.id();

		let mut area = Rect::new(0, 0, 80, 24);
		let mut buf = Buffer::empty(area);

		let mut entity_mut = world.entity_mut(entity);
		Paragraph::render(&mut area, &mut buf, &mut entity_mut).unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("body text");
	}

	#[test]
	fn tuple_renderable_calls_all() {
		let mut world = World::new();
		let entity = world
			.spawn((Heading1, children![(TextNode::new("Title"),)]))
			.id();

		let mut area = Rect::new(0, 0, 80, 24);
		let mut buf = Buffer::empty(area);

		let mut entity_mut = world.entity_mut(entity);
		<(Heading,)>::render(&mut area, &mut buf, &mut entity_mut).unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("Title");
	}

	#[test]
	fn skips_entity_without_component() {
		let mut world = World::new();
		let entity = world.spawn(TextNode::new("not a heading")).id();

		let mut area = Rect::new(0, 0, 80, 24);
		let mut buf = Buffer::empty(area);

		// Should not panic, just skip
		let mut entity_mut = world.entity_mut(entity);
		<(Heading, Paragraph)>::render(&mut area, &mut buf, &mut entity_mut)
			.unwrap();
	}
}
