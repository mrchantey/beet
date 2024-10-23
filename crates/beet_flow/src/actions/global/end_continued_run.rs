use crate::prelude::*;
use bevy::prelude::*;


/// Whenever [`OnRun`] is called, this observer ensures no children are running
/// Only recurses children that have [`Running`] and do not have [`NoInterrupt`]
pub fn end_continued_run(
	trigger: Trigger<OnRunResult>,
	mut commands: Commands,
	running: Query<Entity, With<Running>>,
) {
	if let Some(entity) = running.get(trigger.entity()).ok() {
		// log::info!("end_continued_run: {entity}");
		commands.entity(entity).remove::<Running>();
	}
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
		world.add_observer(end_continued_run);

		let entity = world
			.spawn(Running)
			.flush_trigger(OnRunResult::success())
			.id();

		expect(&world).not().to_have_component::<Running>(entity)?;

		Ok(())
	}
}
