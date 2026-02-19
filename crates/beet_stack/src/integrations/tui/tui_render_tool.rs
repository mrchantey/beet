//! TUI-specific render tool that maintains stateful card entities.
//!
//! Unlike the markdown render tool which despawns content after rendering,
//! the TUI render tool manages a single active card entity marked with
//! [`CurrentCard`]. When a new card is rendered, the previous one is
//! despawned.

use crate::prelude::*;
use beet_core::prelude::*;

/// Creates a TUI rendering tool for stateful card display.
///
/// This tool responds to [`RenderRequest`]s by:
/// 1. Despawning any previous [`CurrentCard`] entity
/// 2. Marking the new content entity with [`CurrentCard`] for the TUI draw system
/// 3. Returning an empty success response
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = StackPlugin::world();
/// // tui_server() already includes a tui_render_tool as a child
/// world.spawn((
///     tui_server(),
///     children![
///         card("home", || Heading1::with_text("Welcome")),
///     ]
/// )).flush();
/// ```
pub fn tui_render_tool() -> impl Bundle {
	(
		RenderToolMarker,
		PathPartial::new("render-tui"),
		async_tool(tui_render_system),
	)
}

/// System that handles TUI rendering requests by despawning the
/// previous card and marking the new content entity with
/// [`CurrentCard`].
async fn tui_render_system(cx: AsyncToolIn<RenderRequest>) -> Result<Response> {
	let card_entity = cx.input.entity;
	let world = cx.tool.world();

	// Despawn previous CurrentCard and mark the new entity
	world
		.with_then(move |world: &mut World| {
			let prev = world
				.query_filtered::<Entity, With<CurrentCard>>()
				.single(world)
				.ok();
			if let Some(prev_entity) = prev {
				world.entity_mut(prev_entity).despawn();
			}
			world.entity_mut(card_entity).insert(CurrentCard);
		})
		.await;

	Response::ok().xok()
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	async fn spawns_card_with_current_marker() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((router(), children![
				tui_render_tool(),
				card("test", || Paragraph::with_text("test content")),
			]))
			.flush();

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("test"))
			.await
			.unwrap();

		// Check that a CurrentCard entity exists
		let current = world
			.query_filtered::<Entity, With<CurrentCard>>()
			.single(&world);
		current.xpect_ok();
	}

	#[beet_core::test]
	async fn despawns_previous_card_on_new_render() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((router(), children![
				tui_render_tool(),
				card("first", || Paragraph::with_text("first")),
				card("second", || Paragraph::with_text("second")),
			]))
			.flush();

		// Render first card
		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("first"))
			.await
			.unwrap();

		let first_current = world
			.query_filtered::<Entity, With<CurrentCard>>()
			.single(&world)
			.unwrap();

		// Render second card
		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("second"))
			.await
			.unwrap();

		let second_current = world
			.query_filtered::<Entity, With<CurrentCard>>()
			.single(&world)
			.unwrap();

		// Should be different entities
		(first_current != second_current).xpect_true();

		// First entity should be despawned entirely
		world.get_entity(first_current).is_err().xpect_true();

		// Second should have CurrentCard
		world
			.entity(second_current)
			.contains::<CurrentCard>()
			.xpect_true();
	}
}
