//! Components for managing long-running action state.
//!
//! Actions that execute over multiple frames need the [`Running`] marker
//! component to indicate they are actively processing. The [`ContinueRun`]
//! component automates adding and removing this marker based on lifecycle events.
use crate::prelude::*;
use beet_core::prelude::*;


/// Automatically manages the [`Running`] component based on lifecycle events.
///
/// When added to an action, this component:
/// - Inserts [`Running`] when [`GetOutcome`] is triggered
/// - Removes [`Running`] when [`Outcome`] is triggered
///
/// This should be added via `#[require(ContinueRun)]` on any action that
/// queries for [`Running`] rather than being added to entities directly.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// #[derive(Component)]
/// #[require(ContinueRun)]
/// struct MyLongRunningAction;
///
/// fn my_system(
///     query: Query<Entity, With<Running>>,
/// ) {
///     for entity in query.iter() {
///         // Process running actions...
///     }
/// }
/// ```
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(RunTimer,InsertOn<GetOutcome,Running>,RemoveOn<Outcome,Running>)]
pub struct ContinueRun;


/// Marker component indicating an action is currently executing.
///
/// This component is typically managed by [`ContinueRun`] rather than being
/// added manually. It is used to query for active long-running actions and
/// is stored as [`SparseSet`](bevy::ecs::component::StorageType::SparseSet)
/// since it is frequently added and removed.
///
/// # Example
///
/// The `Translate` action found in `beet_spatial` demonstrates typical usage:
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
///
/// #[derive(Component)]
/// #[require(ContinueRun)]
/// struct Translate(pub Vec3);
///
/// fn translate(
///     time: Res<Time>,
///     action: Query<(Entity, &Translate),With<Running>>,
///     mut transforms: AgentQuery<&mut Transform>,
/// )-> Result {
///     for (entity, translate) in action.iter() {
///         let mut transform = transforms.get_mut(entity)?;
///         transform.translation += translate.0 * time.delta_secs();
///     }
///   Ok(())
/// }
/// ```
#[derive(Debug, Default, Clone, Copy, Component, PartialEq, Eq, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[require(RunTimer)]
pub struct Running;


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn adds() {
		let mut app = App::new();
		let world = app.world_mut();

		// adds
		let entity = world.spawn(ContinueRun).id();
		world.get::<Running>(entity).xpect_none();
		world.entity_mut(entity).trigger_target(GetOutcome).flush();
		world.get::<Running>(entity).xpect_some();
		world
			.entity_mut(entity)
			.trigger_target(Outcome::Pass)
			.flush();
		world.get::<Running>(entity).xpect_none();
	}
}
