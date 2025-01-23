use crate::prelude::*;
use bevy::prelude::*;


/// Removes [`Running`] from the entity when [`OnRunResult`] is triggered.
/// Also removes [`Running`] from children unless they have a [`NoInterrupt`]
pub fn interrupt_on_run_result(
	trigger: Trigger<OnRunResult>,
	mut commands: Commands,
	running: Populated<Entity, With<Running>>,
	children: Query<&Children>,
	children_should_remove: Populated<
		(),
		(With<Running>, Without<NoInterrupt>),
	>,
) {
	if let Some(entity) = running.get(trigger.entity()).ok() {
		commands.entity(entity).remove::<Running>();
	}
	for child in children.iter_descendants(trigger.entity()) {
		if children_should_remove.contains(child) {
			commands.entity(child).remove::<Running>();
		}
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
		world.add_observer(interrupt_on_run_result);

		let entity = world
			.spawn(Running)
			.flush_trigger(OnRunResult::success())
			.id();

		expect(&world).not().to_have_component::<Running>(entity)?;

		Ok(())
	}
}
