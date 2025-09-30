use crate::prelude::*;
use bevy::prelude::*;


/// Removes [`Running`] from the entity when [`OnResult`] is triggered.
/// Also removes [`Running`] from children unless they have a [`NoInterrupt`].
pub(super) fn interrupt_on_result<T: ResultPayload>(
	ev: On<OnResultAction<T>>,
	mut commands: Commands,
	// names: Query<&Name>,
	children: Query<&Children>,
	should_remove: Populated<(), (With<Running>, Without<NoInterrupt>)>,
) {
	let action = ev.resolve_action();
	if should_remove.contains(action) {
		commands.entity(action).remove::<Running>();
	}
	for child in children
		.iter_descendants(action)
		.filter(|child| should_remove.contains(*child))
	{
		commands.entity(child).remove::<Running>();
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		world
			.spawn(Running::default())
			.with_child(Running::default())
			.flush_trigger(OnResultAction::local(RunResult::Success));

		world.query::<&Running>().iter(&world).count().xpect_eq(0);
	}
}
