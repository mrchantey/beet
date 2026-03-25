use beet_core::prelude::*;

/// Marks the currently active scene entity in a stateful interface
/// like TUI.
///
/// Used by the TUI draw system to know which scene to render. The
/// render tool despawns the previous scene before inserting this on
/// the new one. The [`single_current_scene`] observer acts as a
/// safety net, removing stale markers if anything else inserts this
/// component.
#[derive(Default, Component)]
pub struct CurrentScene;

/// Safety-net observer that removes [`CurrentScene`] from all other
/// entities when a new one is inserted, ensuring at most one active
/// scene at a time.
pub(crate) fn single_current_scene(
	insert: On<Insert, CurrentScene>,
	query: Query<Entity, With<CurrentScene>>,
	mut commands: Commands,
) {
	for entity in query.iter() {
		if entity != insert.entity {
			commands.entity(entity).remove::<CurrentScene>();
		}
	}
}
