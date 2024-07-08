use crate::prelude::*;
use bevy::prelude::*;


/// A component that turns into an [`OnRun`] event when the world observes [`trigger_run_on_spawn`], useful for scene-based workflows
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct RunOnSpawn;
pub fn trigger_run_on_spawn(
	trigger: Trigger<OnAdd, RunOnSpawn>,
	mut commands: Commands,
) {
	commands.trigger_targets(OnRun, trigger.entity());
}


#[cfg(test)]
mod test {
	use super::RunOnSpawn;
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();
		world.observe(trigger_run_on_spawn);
		let func = observe_triggers::<OnRun>(&mut world);

		world.spawn(RunOnSpawn);
		expect(&func).not().to_have_been_called()?;
		world.flush();
		expect(&func).to_have_been_called_times(1)?;
		world.flush();
		expect(&func).to_have_been_called_times(1)?;
		Ok(())
	}
}
