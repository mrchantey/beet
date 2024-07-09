use crate::prelude::*;
use bevy::prelude::*;


/// A component that turns into an [`OnRun`] event on add, 
/// useful for scene-based workflows
/// This will likely be deprecated if/when bsn observers are implemented
#[derive(Default, Action, Reflect)]
#[reflect(Default, Component)]
#[systems(run_on_spawn.in_set(PreTickSet))]
pub struct RunOnSpawn;
// pub fn trigger_run_on_spawn(
// 	trigger: Trigger<OnAdd, RunOnSpawn>,
// 	mut commands: Commands,
// ) {
// 	commands.trigger_targets(OnRun, trigger.entity());
// }

/// cannot use observers because we need to wait for children to be built
/// which happens after component add
pub fn run_on_spawn(
	mut commands: Commands,
	query: Query<Entity, Added<RunOnSpawn>>,
) {
	for entity in query.iter() {
		commands.trigger_targets(OnRun, entity);
	}
}


#[cfg(test)]
mod test {
	use super::RunOnSpawn;
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();
		// world.observe(trigger_run_on_spawn);
		let func = observe_triggers::<OnRun>(&mut world);

		world.spawn(RunOnSpawn);
		expect(&func).not().to_have_been_called()?;
		world.run_system_once(run_on_spawn);
		world.flush();
		expect(&func).to_have_been_called_times(1)?;
		world.flush();
		expect(&func).to_have_been_called_times(1)?;
		Ok(())
	}
}
