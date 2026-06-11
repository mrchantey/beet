use beet_core::prelude::*;

/// Marks the currently active page entity in a stateful interface
/// like TUI.
///
/// Used by the TUI draw system to know which page to render. The
/// render action despawns the previous page before inserting this on
/// the new one. The [`single_current_page`] observer acts as a
/// safety net, removing stale markers if anything else inserts this
/// component.
#[derive(Default, Component)]
pub struct CurrentPage;

/// Safety-net observer that removes [`CurrentPage`] from all other
/// entities when a new one is inserted, ensuring at most one active
/// page at a time.
///
/// Only registered by the std-only [`NavigatorPlugin`].
#[cfg(feature = "std")]
pub(crate) fn single_current_page(
	insert: On<Insert, CurrentPage>,
	query: Query<Entity, With<CurrentPage>>,
	mut commands: Commands,
) {
	for entity in query.iter() {
		if entity != insert.entity {
			commands.entity(entity).remove::<CurrentPage>();
		}
	}
}
