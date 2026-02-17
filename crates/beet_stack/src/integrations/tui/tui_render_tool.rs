//! TUI-specific render tool that maintains stateful card entities.
//!
//! Unlike the markdown render tool which spawns/despawns content per request,
//! the TUI render tool maintains persistent card content entities marked with
//! [`CurrentCard`] for the draw system to render.

use crate::prelude::*;
use beet_core::prelude::*;

/// Creates a TUI rendering tool for stateful card display.
///
/// This tool responds to [`RenderRequest`]s by:
/// 1. Spawning the card's content using its [`CardSpawner`]
/// 2. Marking it with [`CurrentCard`] for the TUI draw system
/// 3. Returning an empty success response
///
/// The spawned content persists until a new card is rendered, at which
/// point the [`single_current_card`] observer removes the old marker.
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = StackPlugin::world();
/// world.spawn((
///     tui_server(),
///     children![
///         tui_render_tool(),
///         card("home", || Heading1::with_text("Welcome")),
///     ]
/// )).flush();
/// ```
pub fn tui_render_tool() -> impl Bundle {
	(
		RenderToolMarker,
		PathPartial::new("render-tui"),
		tool(tui_render_system),
	)
}

/// System that handles TUI rendering requests by spawning card content
/// and marking it with [`CurrentCard`].
async fn tui_render_system(
	cx: AsyncToolContext<RenderRequest>,
) -> Result<Response> {
	let handler = cx.input.handler;
	let world = cx.tool.world();

	// Spawn the card content and mark it as current
	let _card_entity = world
		.with_then(move |world: &mut World| -> Result<Entity> {
			// Get the CardSpawner from the handler entity
			let spawner = world
				.entity(handler)
				.get::<CardSpawner>()
				.ok_or_else(|| {
					bevyhow!(
						"Card handler entity missing CardSpawner component"
					)
				})?
				.clone();

			// Spawn the card content
			let card_entity = spawner.spawn(world);

			// Mark it as the current card for TUI rendering
			world.entity_mut(card_entity).insert(CurrentCard);

			Ok(card_entity)
		})
		.await?;

	// Return empty success response
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
	async fn replaces_previous_current_card() {
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

		// First should no longer have CurrentCard
		world
			.entity(first_current)
			.contains::<CurrentCard>()
			.xpect_false();

		// Second should have CurrentCard
		world
			.entity(second_current)
			.contains::<CurrentCard>()
			.xpect_true();
	}
}
