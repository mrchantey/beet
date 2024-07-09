use crate::prelude::*;
use bevy::prelude::*;

pub type ContinueRun = InsertWhileRunning<Running>;


/// 1. Adds the provided component when [`OnRun`] is called
/// 2. Removes the component when [`OnRunResult`] is called
#[derive(Default, Action, Reflect)]
#[reflect(Default, Component)]
#[observers(on_start_running::<T>, on_stop_running::<T>)]
pub struct InsertWhileRunning<T: Default + GenericActionComponent>(pub T);

fn on_start_running<T: Default + GenericActionComponent>(
	trigger: Trigger<OnRun>,
	query: Query<&InsertWhileRunning<T>>,
	mut commands: Commands,
) {
	let action = query
		.get(trigger.entity())
		.expect(expect_action::NO_ACTION_COMP);
	commands.entity(trigger.entity()).insert(action.0.clone());
}
fn on_stop_running<T: Default + GenericActionComponent>(
	trigger: Trigger<OnRunResult>,
	mut commands: Commands,
) {
	commands.entity(trigger.entity()).remove::<T>();
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();

		let entity = world
			.spawn(ContinueRun::default())
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(3)?;
		expect(&world).to_have_component::<Running>(entity)?;
		world
			.entity_mut(entity)
			.flush_trigger(OnRunResult::success());
		expect(&world).not().to_have_component::<Running>(entity)?;
		Ok(())
	}
}
