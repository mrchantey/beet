//! Interrupt handling for running actions.
//!
//! This module provides automatic interruption of running actions when new
//! events are triggered. The [`NoInterrupt`] component can be used to prevent
//! automatic interruption.
use crate::prelude::*;
use beet_core::prelude::*;

/// Prevents automatic interruption of this action.
///
/// By default, when a new [`RunEvent`] is triggered on an action, all running
/// descendant actions have their [`Running`] component removed. Adding this
/// component prevents that automatic removal.
///
/// Note that [`NoInterrupt`] only prevents interruption from *parent* events.
/// Direct triggers on the entity itself will still remove [`Running`].
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = ControlFlowPlugin::world();
/// // This child will keep running even if parent is re-triggered
/// world.spawn(children![(NoInterrupt, Running)]);
/// ```
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct NoInterrupt;


/// Removes [`Running`] from children when a [`RunEvent`] is triggered.
///
/// This does not remove [`Running`] from the action entity itself, as it may
/// have just been added. Actions with [`NoInterrupt`] are skipped.
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


/// Removes [`Running`] from the entity and its children when an [`EndEvent`] is triggered.
///
/// The [`Running`] component is always removed from the event target. For children,
/// removal is skipped if they have [`NoInterrupt`].
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
