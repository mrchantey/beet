use beet_core::prelude::*;

/// Marks the currently active card entity in a stateful interface
/// like TUI.
///
/// Used by the TUI draw system to know which card to render. The
/// [`tui_render_tool`](crate::prelude::tui_render_tool) despawns
/// the previous card before inserting this on the new one. The
/// [`single_current_card`] observer acts as a safety net, removing
/// stale markers if anything else inserts this component.
#[derive(Default, Component)]
pub struct CurrentCard;

/// Safety-net observer that removes [`CurrentCard`] from all other
/// entities when a new one is inserted, ensuring at most one active
/// card at a time.
pub(super) fn single_current_card(
	insert: On<Insert, CurrentCard>,
	query: Query<Entity, With<CurrentCard>>,
	mut commands: Commands,
) {
	for entity in query.iter() {
		if entity != insert.entity {
			commands.entity(entity).remove::<CurrentCard>();
		}
	}
}
