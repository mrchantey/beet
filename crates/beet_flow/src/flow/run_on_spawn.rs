use crate::prelude::*;
use bevy::prelude::*;



/// Sometimes its useful to run an action by spawning an entity,
/// for example if you want to run on the next frame to avoid
/// infinite loops or await updated world state.
///
/// This component is SparsSet as it is frequently added and removed.
#[derive(Debug, Clone, Component)]
#[component(storage = "SparseSet")]
pub struct RunOnSpawn<T: RunPayload = ()> {
	pub payload: T,
}

impl Default for RunOnSpawn<()> {
	fn default() -> Self { Self { payload: () } }
}

// we use a system instead of observer to avoid infinite loops
pub fn run_on_spawn(
	mut commands: Commands,
	query: Populated<(Entity, &RunOnSpawn)>,
) {
	for (entity, run_on_spawn) in query.iter() {
		commands
			.entity(entity)
			.remove::<RunOnSpawn>()
			.trigger(OnRunAction::local(run_on_spawn.payload.clone()));
	}
}
