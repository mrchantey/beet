use crate::prelude::*;
use beet_core::prelude::*;

/// Mark a behavior as uninterruptible, the `Running` component
/// will only be removed if [`End`] is called on it,
/// either directly or via event propagation.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct NoInterrupt;


/// removes [`Running`] from children when [`T`] is called, unless they have a [`NoInterrupt`].
/// Unlike [`interrupt_on_end`], this does not remove the `Running` component
/// from the action entity, as it may have been *just added*.
pub(crate) fn interrupt_on_run<T: RunEvent>(
	ev: On<T>,
	mut commands: Commands,
	running: Populated<Entity, (With<Running>, Without<NoInterrupt>)>,
	children: Populated<&Children>,
) {
	let action = ev.target();
	for child in children.iter_descendants(action) {
		if running.contains(child) {
			commands.entity(child).remove::<Running>();
		}
	}
}


/// Removes [`Running`] from the entity when [`End`] is triggered.
/// Also removes [`Running`] from children unless they have a [`NoInterrupt`].
pub(crate) fn interrupt_on_end<T: EndEvent>(
	ev: On<T>,
	mut commands: Commands,
	children: Query<&Children>,
	running: Populated<Entity, With<Running>>,
	no_interrupt: Query<(), With<NoInterrupt>>,
) {
	let action = ev.target();
	// 1. always remove from this entity
	if running.contains(action) {
		commands.entity(action).remove::<Running>();
	}
	// 2. only remove from children if they don't have NoInterrupt
	for child in children.iter_descendants(action) {
		if running.contains(child) && !no_interrupt.contains(child) {
			commands.entity(child).remove::<Running>();
		}
	}
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;



	#[test]
	fn interrupt_on_run() {
		let mut world = ControlFlowPlugin::world();

		world
			.spawn(children![Running])
			.trigger_target(GetOutcome)
			.flush();
		world.query_once::<&Running>().len().xpect_eq(0);
	}
	#[test]
	fn no_interrupt_on_run() {
		let mut world = ControlFlowPlugin::world();
		world
			.spawn(children![(NoInterrupt, Running)])
			.trigger_target(GetOutcome)
			.flush();
		world.query_once::<&Running>().len().xpect_eq(1);
	}

	#[test]
	fn interrupt_on_end() {
		let mut world = ControlFlowPlugin::world();

		world
			.spawn((Running, children![Running, (NoInterrupt, Running)]))
			.trigger_target(Outcome::Pass)
			.flush();

		// removes from parent and first child, but not NoInterrupt child
		world.query_once::<&Running>().len().xpect_eq(1);
	}
	#[test]
	fn interrupt_on_end_with_no_interrupt() {
		let mut world = ControlFlowPlugin::world();
		// NoInterrupt only prevents interruption from *parent*, not direct triggers
		world
			.spawn((NoInterrupt, Running))
			.trigger_target(Outcome::Pass)
			.flush();
		// Direct trigger on entity still removes Running
		world.query_once::<&Running>().len().xpect_eq(0);
	}
}
