use crate::prelude::*;
use bevy::prelude::*;


// pub fn name_or_entity(names: &Query<&Name>, entity: Entity) -> String {
// 	names
// 		.get(entity)
// 		.map(|name| name.to_string())
// 		.unwrap_or(entity.to_string())
// }

/// Removes [`Running`] from the entity when [`OnRunResult`] is triggered.
/// Also removes [`Running`] from children unless they have a [`NoInterrupt`]
pub fn interrupt_on_run_result(
	trigger: Trigger<OnRunResult>,
	mut commands: Commands,
	// names: Query<&Name>,
	children: Query<&Children>,
	should_remove: Populated<(), (With<Running>, Without<NoInterrupt>)>,
) {
	let parent = trigger.entity();
	// println!("interrupting {}", name_or_entity(&names, parent));

	if should_remove.contains(parent) {
		// println!("stopped entity: {}", name_or_entity(&names, parent));
		commands.entity(parent).remove::<Running>();
	}

	for child in children
		.iter_descendants(parent)
		.filter(|child| should_remove.contains(*child))
	{
		// println!("stopped child: {}", name_or_entity(&names, child));
		commands.entity(child).remove::<Running>();
	}
	// for child in children
	// 	.iter_descendants(parent)
	// 	.filter(|child| !should_remove.contains(*child))
	// {
	// 	println!("not stopping child: {}", name_or_entity(&names, child));
	// }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		world.add_observer(interrupt_on_run_result);

		world
			.spawn(Running)
			.with_child(Running)
			.flush_trigger(OnRunResult::success());

		expect(world.query::<&Running>().iter(&world).count()).to_be(0);
	}
}
