//! TUI rendering via [`TuiRenderer`] and [`CardWalker`].
//!
//! The [`draw_system`] is an exclusive system that walks the card
//! tree via [`CardWalker`] and renders each entity using the
//! visitor-based [`TuiRenderer`].
use crate::prelude::*;
use beet_core::prelude::*;
use bevy_ratatui::RatatuiContext;
use ratatui::Frame;

/// Renders the current card's content tree into the terminal each frame.
///
/// Walks the card tree via [`CardWalker`] and renders using
/// [`TuiRenderer`], which implements [`CardVisitor`] for unified
/// rendering of all content types.
pub(super) fn draw_system(world: &mut World) -> Result {
	let card_entity = match world
		.query_filtered::<Entity, With<CurrentCard>>()
		.single(world)
	{
		Ok(entity) => entity,
		Err(_) => {
			// not selected yet, just write 'loading'
			world.resource_mut::<RatatuiContext>().draw(|frame| {
				let text = ratatui::text::Text::from("loading..");
				frame.render_widget(text, frame.area());
			})?;

			return Ok(());
		}
	};

	world.resource_scope(
		|world: &mut World, mut context: Mut<RatatuiContext>| {
			bevy_draw(&mut context, |frame| {
				let area = frame.area();
				let buf = frame.buffer_mut();

				world
					.run_system_once_with(
						|In((entity, area)): In<(
							Entity,
							ratatui::prelude::Rect,
						)>,
						 walker: CardWalker| {
							// Create a temporary buffer, render into it,
							// then return it for merging
							let mut inner_buf =
								ratatui::buffer::Buffer::empty(area);
							let mut renderer =
								TuiRenderer::new(area, &mut inner_buf);
							walker.walk_card(&mut renderer, entity);
							inner_buf
						},
						(card_entity, area),
					)
					.map(|inner_buf| {
						// Merge the inner buffer into the frame buffer
						let area = inner_buf.area;
						for row in area.y..area.y + area.height {
							for col in area.x..area.x + area.width {
								*buf.cell_mut((col, row)).unwrap() =
									inner_buf[(col, row)].clone();
							}
						}
					})
					.map_err(|err| bevyhow!("{err}"))?;
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


#[cfg(test)]
mod test {
	use super::*;
	use ratatui::buffer::Buffer;
	use ratatui::prelude::Rect;

	#[test]
	fn heading_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Heading1, children![TextNode::new(
				"Hello World"
			)])]))
			.id();

		let area = Rect::new(0, 0, 80, 24);
		let buf = world
			.run_system_once_with(
				|In((entity, area)): In<(Entity, Rect)>, walker: CardWalker| {
					let mut buf = Buffer::empty(area);
					let mut renderer = TuiRenderer::new(area, &mut buf);
					walker.walk_card(&mut renderer, entity);
					buf
				},
				(entity, area),
			)
			.unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("Hello World");
	}

	#[test]
	fn paragraph_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![TextNode::new(
				"body text"
			)])]))
			.id();

		let area = Rect::new(0, 0, 80, 24);
		let buf = world
			.run_system_once_with(
				|In((entity, area)): In<(Entity, Rect)>, walker: CardWalker| {
					let mut buf = Buffer::empty(area);
					let mut renderer = TuiRenderer::new(area, &mut buf);
					walker.walk_card(&mut renderer, entity);
					buf
				},
				(entity, area),
			)
			.unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("body text");
	}

	#[test]
	fn skips_nested_card() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				(Paragraph, children![TextNode::new("visible")]),
				(CardTool, children![(Paragraph, children![TextNode::new(
					"hidden"
				)])]),
			]))
			.id();

		let area = Rect::new(0, 0, 80, 24);
		let buf = world
			.run_system_once_with(
				|In((entity, area)): In<(Entity, Rect)>, walker: CardWalker| {
					let mut buf = Buffer::empty(area);
					let mut renderer = TuiRenderer::new(area, &mut buf);
					walker.walk_card(&mut renderer, entity);
					buf
				},
				(entity, area),
			)
			.unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.as_str().xpect_contains("visible");
		rendered.contains("hidden").xpect_false();
	}
}
