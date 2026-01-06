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
	mut running: Populated<&mut Running, Without<NoInterrupt>>,
	children: Populated<&Children>,
) {
	let action = ev.action();
	for child in children.iter_descendants(action) {
		if let Ok(mut running) = running.get_mut(child) {
			running.retain(&mut commands, child, ev.agent());
		}
	}
}


/// Removes [`Running`] from the entity when [`End`] is triggered.
/// Also removes [`Running`] from children unless they have a [`NoInterrupt`].
pub(crate) fn interrupt_on_end<T: EndEvent>(
	ev: On<T>,
	mut commands: Commands,
	children: Query<&Children>,
	mut running: Populated<(&mut Running, Option<&NoInterrupt>)>,
) {
	// 1. always remove from this entity
	if let Ok((mut running, _)) = running.get_mut(ev.action()) {
		running.retain(&mut commands, ev.action(), ev.agent());
	}
	let action = ev.action();
	// 2. only remove from children if NoInterrupt
	for child in children.iter_descendants(action) {
		if let Ok((mut running, no_interrupt)) = running.get_mut(child)
			&& no_interrupt.is_none()
		{
			running.retain(&mut commands, child, ev.agent());
		}
	}
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;


	// ads running to this entity, with the root
	// as its agent
	fn root_running() -> impl Bundle {
		(
			ContinueRun,
			OnSpawn::new(|entity| {
				let mut root = entity.id();
				while let Some(ChildOf(next)) =
					entity.world().entity(root).get()
				{
					root = *next;
				}
				entity.insert(Running(vec![root]));
			}),
		)
	}

	#[test]
	fn interrupt_on_run() {
		let mut world = ControlFlowPlugin::world();

		world
			.spawn(children![root_running()])
			.trigger_target(GetOutcome)
			.flush();
		world.query_once::<&Running>().len().xpect_eq(0);
	}
	#[test]
	fn no_interrupt_on_run() {
		let mut world = ControlFlowPlugin::world();
		world
			.spawn(children![(NoInterrupt, root_running())])
			.trigger_target(GetOutcome)
			.flush();
		world.query_once::<&Running>().len().xpect_eq(1);
	}

	#[test]
	fn interrupt_on_end() {
		let mut world = ControlFlowPlugin::world();

		world
			.spawn((root_running(), children![
				root_running(),
				(NoInterrupt, root_running())
			]))
			.trigger_target(Outcome::Pass)
			.flush();

		// removes from parent and first child
		world.query_once::<&Running>().len().xpect_eq(1);
	}
	#[test]
	fn interrupt_on_end_with_no_interrupt() {
		let mut world = ControlFlowPlugin::world();
		world
			.spawn((NoInterrupt, root_running()))
			.trigger_target(Outcome::Pass)
			.flush();
		world.query_once::<&Running>().len().xpect_eq(0);
	}
}
