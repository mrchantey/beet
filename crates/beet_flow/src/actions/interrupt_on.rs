use crate::prelude::*;
use beet_core::prelude::*;

/// Mark a behavior as uninterruptible, the `Running` component
/// will only be removed if [`End`] is called on it,
/// either directly or via event propagation.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct NoInterrupt;


/// removes [`Running`] from children when [`Run`] is called, unless they have a [`NoInterrupt`].
/// Unlike [`interrupt_on_result`], this does not remove the `Running` component
/// from the action entity, as it may have been *just added*.
pub(crate) fn interrupt_on_run<T: 'static + Send + Sync>(
	ev: On<Run<T>>,
	mut commands: Commands,
	should_remove: Populated<(), (With<Running>, Without<NoInterrupt>)>,
	children: Populated<&Children>,
) {
	let action = ev.target();
	for child in children
		.iter_descendants(action)
		.filter(|child| should_remove.contains(*child))
	{
		commands.entity(child).remove::<Running>();
	}
}


/// Removes [`Running`] from the entity when [`End`] is triggered.
/// Also removes [`Running`] from children unless they have a [`NoInterrupt`].
pub(crate) fn interrupt_on_end<
	T: 'static + Send + Sync,
	E: 'static + Send + Sync,
>(
	ev: On<End<T, E>>,
	mut commands: Commands,
	children: Query<&Children>,
	should_remove: Populated<(), (With<Running>, Without<NoInterrupt>)>,
) {
	let action = ev.target();
	// 1. always remove from this entity
	if should_remove.contains(action) {
		commands.entity(action).remove::<Running>();
	}
	// 2. only remove from children if NoInterrupt
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
	use beet_core::prelude::EntityWorldMutEntityTargetTriggerExt;
	use beet_core::prelude::IntoWorldMutExt;
	use bevy::prelude::*;
	use sweet::prelude::*;

	fn setup() -> World {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		std::mem::take(app.world_mut())
	}

	#[test]
	fn interrupt_on_run() {
		let mut world = setup();
		world.spawn(children![Running]).trigger_target(RUN);
		world.query_once::<&Running>().len().xpect_eq(0);
	}
	#[test]
	fn no_interrupt_on_run() {
		let mut world = setup();
		world
			.spawn(children![(NoInterrupt, Running)])
			.trigger_target(RUN);
		world.query_once::<&Running>().len().xpect_eq(1);
	}

	#[test]
	fn interrupt_on_end() {
		let mut world = setup();

		world
			.spawn((Running, children![Running, (NoInterrupt, Running)]))
			.trigger_target(SUCCESS);

		// removes from parent and first child
		world.query_once::<&Running>().len().xpect_eq(1);
	}
	#[test]
	fn interrupt_on_end_with_no_interrupt() {
		let mut world = setup();
		world.spawn((NoInterrupt, Running)).trigger_target(SUCCESS);
		// leaves parent
		world.query_once::<&Running>().len().xpect_eq(1);
	}
}
