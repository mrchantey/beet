use crate::prelude::*;
use bevy::ecs::component::ComponentHooks;
use bevy::ecs::component::StorageType;
use bevy::prelude::*;

/// Trigger `OnRunResult` immediately when this action runs
#[derive(Default, Deref, DerefMut, Reflect)]
#[reflect(Default, Component)]
pub struct EndOnRun(pub RunResult);

impl EndOnRun {
	pub fn success() -> Self { Self(RunResult::Success) }
	pub fn failure() -> Self { Self(RunResult::Failure) }
}

impl Component for EndOnRun {
	const STORAGE_TYPE: StorageType = StorageType::Table;
	fn register_component_hooks(hooks: &mut ComponentHooks) {
		hooks.on_add(|mut world, entity, _| {
			ActionObserverHooks::new::<EndOnRun>()
				.add_observers(end_on_run)
				.build(world.commands(), entity);
		});
		hooks.on_remove(ActionObserverHooks::cleanup::<EndOnRun>);
	}
}

fn end_on_run(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	query: Query<&EndOnRun>,
) {
	if let Ok(end_on_run) = query.get(trigger.entity()) {
		commands
			.trigger_targets(OnRunResult::new(**end_on_run), trigger.entity());
	}
}


#[cfg(test)]
mod test {
	use super::EndOnRun;
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();
		let func = observe_run_results(&mut world);

		expect(world.entities().len()).to_be(1)?;
		let entity = world.spawn(EndOnRun::failure()).id();
		expect(world.entities().len()).to_be(2)?;
		world.flush();
		expect(world.entities().len()).to_be(3)?;
		expect(&func).not().to_have_been_called()?;
		world.trigger_targets(OnRun, entity);
		world.flush();
		expect(&func).to_have_been_called()?;
		expect(&func).to_have_returned_nth_with(0, &RunResult::Failure)?;
		Ok(())
	}
	#[test]
	fn works_with_run_on_spawn() -> Result<()> {
		let mut world = World::new();
		world.observe(trigger_run_on_spawn);
		let func = observe_run_results(&mut world);

		world.spawn((RunOnSpawn, EndOnRun::failure()));
		world.flush();
		expect(world.entities().len()).to_be(4)?;
		world.flush();
		expect(&func).to_have_been_called()?;
		expect(&func).to_have_returned_nth_with(0, &RunResult::Failure)?;
		Ok(())
	}
}
