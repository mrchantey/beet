use beet_core::prelude::*;

/// Marks the currently active card entity in a stateful interface like TUI.
///
/// Used by the TUI server's draw system to know which card to render.
/// Only one entity should have this component at a time, enforced by
/// the [`single_current_card`] observer.
#[derive(Default, Component)]
pub struct CurrentCard;

/// Observer that ensures only one card at a time has the [`CurrentCard`] component.
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
