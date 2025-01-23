use crate::prelude::*;
use bevy::prelude::*;


/// Removes [`Running`] from the entity when [`OnRunResult`] is triggered.
/// Also removes [`Running`] from children unless they have a [`NoInterrupt`]
pub fn interrupt_on_run_result(
	trigger: Trigger<OnRunResult>,
	mut commands: Commands,
	children: Query<&Children>,
	should_remove: Populated<(), (With<Running>, Without<NoInterrupt>)>,
) {
	let entity = trigger.entity();

	if should_remove.contains(entity) {
		println!("stopped entity: {}",entity);
		commands.entity(entity).remove::<Running>();
	}
	
	for child in children.iter_descendants(entity) {
		println!("stopped child: {}",child);
		if should_remove.contains(child) {
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

		world
			.spawn(Running)
			.with_child(Running)
			.flush_trigger(OnRunResult::success());

		expect(world.query::<&Running>().iter(&world).count()).to_be(0)?;

		// expect(&world).not().to_have_component::<Running>(entity)?;

		Ok(())
	}
}
