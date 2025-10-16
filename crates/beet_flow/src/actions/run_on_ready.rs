use crate::prelude::*;
use beet_core::prelude::*;

/// Triggers [`GetOutcome`] when a [`Ready`] event is received.
#[action(run_on_ready)]
#[derive(Debug, Default, Component)]
pub struct RunOnReady;
fn run_on_ready(ev: On<Ready>, mut commands: Commands) {
	if ev.event_target() == ev.trigger().original_event_target {
		commands
			.entity(ev.event_target())
			.trigger_target(GetOutcome);
	}
}

#[action(request_child_ready, handle_child_ready)]
#[derive(Debug, Default, Component)]
pub struct ReadyOnChildrenReady {
	/// The number of [`ReadyAction`] descendants that have
	/// triggered [`Ready`].
	pub num_ready: u32,
	/// The number of children with a [`ReadyAction`] component.
	/// This is recalculated on each [`GetReady`] triggered.
	pub num_actions: u32,
}

/// For each descendant with a [`ReadyAction`] component,
/// trigger a [`GetReady`] event, and reset the counters.
fn request_child_ready(
	ev: On<GetReady>,
	mut commands: Commands,
	mut action: Query<&mut ReadyOnChildrenReady>,
	children: Query<&Children>,
	ready_actions: Query<Entity, With<ReadyAction>>,
) -> Result {
	let entities = children
		.iter_descendants(ev.event_target())
		.filter_map(|child| ready_actions.get(child).ok())
		.collect::<Vec<_>>();
	let mut run_on_ready = action.get_mut(ev.event_target())?;
	run_on_ready.num_actions = entities.len() as u32;
	run_on_ready.num_ready = 0;
	if entities.is_empty() {
		// no child will be spawned, we're immediately ready
		commands.entity(ev.event_target()).trigger(Ready);
	} else {
		// trigger each child to become ready
		for entity in entities.iter() {
			commands.entity(*entity).trigger(GetReady);
		}
	}
	Ok(())
}

fn handle_child_ready(
	ev: On<Ready>,
	mut commands: Commands,
	mut action: Query<&mut ReadyOnChildrenReady>,
) -> Result {
	// only handle bubbled up events
	if ev.event_target() == ev.trigger().original_event_target {
		return Ok(());
	}

	let mut action = action.get_mut(ev.event_target())?;
	action.num_ready += 1;
	if action.num_ready == action.num_actions {
		commands.entity(ev.event_target()).trigger(Ready);
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn no_children() {
		let store = Store::default();
		let mut world = World::new();
		world
			.spawn((ReadyOnChildrenReady::default(), children![()]))
			.observe(move |_: On<Ready>| {
				store.set(true);
			})
			.trigger(GetReady)
			.flush();

		store.get().xpect_eq(true);
	}

	#[sweet::test]
	async fn works() {
		let store = Store::default();
		let mut world = (MinimalPlugins, AsyncPlugin).into_world();
		world
			.spawn((RunOnReady, ReadyOnChildrenReady::default(), children![
				ReadyAction::new(async |_| {
					beet_core::exports::futures_lite::future::yield_now().await;
				}),
				ReadyAction::new(async |_| {
					beet_core::exports::futures_lite::future::yield_now().await;
				})
			]))
			.observe_any(move |_: On<GetOutcome>| {
				store.set(true);
			})
			.trigger(GetReady)
			.flush();
		AsyncRunner::flush_async_tasks(&mut world).await;

		store.get().xpect_eq(true);
	}
}
