use crate::prelude::*;
use beet_core::prelude::*;

/// Waits for all [`ReadyAction`] descendants to complete before triggering [`Outcome::Pass`].
///
/// On [`GetOutcome`], this action finds all [`ReadyAction`] descendants from the root
/// of the behavior tree and triggers [`GetReady`] on each. Once all have responded with
/// [`Ready`], it triggers [`Outcome::Pass`].
///
/// This is useful for dynamically spawned trees that have async initialization.
/// Use [`EndInDuration`] as a sibling for timeout behavior.
///
/// ## Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = ControlFlowPlugin::world();
/// world.spawn((
///     Sequence,
///     children![
///         AwaitReady::default(),
///         EndWith(Outcome::Pass),
///     ]
/// ));
/// ```
#[action(await_ready_start)]
#[derive(Debug, Default, Component)]
#[require(ContinueRun)]
pub struct AwaitReady {
	/// The number of [`ReadyAction`] descendants that have triggered [`Ready`].
	pub num_ready: u32,
	/// The number of descendants with a [`ReadyAction`] component.
	pub num_actions: u32,
	/// Entities we're waiting for Ready signals from.
	pub pending: HashSet<Entity>,
}

fn await_ready_start(
	ev: On<GetOutcome>,
	mut commands: Commands,
	mut action: Query<&mut AwaitReady>,
	agents: AgentQuery,
	children: Query<&Children>,
	ready_actions: Query<Entity, With<ReadyAction>>,
) -> Result {
	let target = ev.target();
	// find the root of the tree to search all descendants
	let root = agents.parents.root_ancestor(target);

	let entities: HashSet<Entity> = children
		.iter_descendants(root)
		.filter_map(|child| ready_actions.get(child).ok())
		.collect();

	let mut await_ready = action.get_mut(target)?;
	await_ready.num_actions = entities.len() as u32;
	await_ready.num_ready = 0;
	await_ready.pending = entities.clone();

	if entities.is_empty() {
		// no ReadyActions found, immediately pass
		commands.entity(target).trigger_target(Outcome::Pass);
	} else {
		info!(
			"AwaitReady: waiting for {} actions",
			await_ready.num_actions
		);

		// observe Ready on the root where events bubble to
		commands.entity(root).observe(
			move |ev: On<Ready>,
			      mut commands: Commands,
			      mut action: Query<&mut AwaitReady>| {
				let original = ev.trigger().original_event_target;
				let Ok(mut action) = action.get_mut(target) else {
					return;
				};

				// only count Ready from entities we triggered
				if !action.pending.remove(&original) {
					return;
				}

				action.num_ready += 1;
				info!(
					"AwaitReady: {} / {} ready",
					action.num_ready, action.num_actions
				);

				if action.num_ready == action.num_actions {
					commands.entity(target).trigger_target(Outcome::Pass);
					// despawn the observer
					commands.entity(ev.observer()).despawn();
				}
			},
		);

		// trigger GetReady on each ReadyAction
		for entity in entities.iter() {
			commands.entity(*entity).trigger(GetReady);
		}
	}
	Ok(())
}

/// Marker for entities collect in the [`PostStartup`] schedule,
/// each of which will have a [`GetReady`] triggered on them.
#[derive(Debug, Default, Component)]
pub struct GetReadyOnStartup;

pub fn get_ready_on_startup(
	mut commands: Commands,
	query: Query<Entity, With<GetReadyOnStartup>>,
) {
	for entity in query.iter() {
		commands.entity(entity).trigger(GetReady);
	}
}


/// Triggers [`GetOutcome`] when a [`Ready`] event is received.
#[action(run_on_ready)]
#[derive(Debug, Default, Component)]
pub struct RunOnReady;
fn run_on_ready(ev: On<Ready>, mut commands: Commands) {
	if ev.event_target() == ev.trigger().original_event_target {
		commands
			.entity(ev.event_target())
			.trigger_target(GetOutcome);
	} else {
		warn!("RunOnReady mismatch.. todo how do we handle this?");
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
	info!("actions ready: 0 / {}", run_on_ready.num_actions);
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
	info!(
		"actions ready: {} / {}",
		action.num_ready, action.num_actions
	);
	if action.num_ready == action.num_actions {
		commands.entity(ev.event_target()).trigger(Ready);
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn await_ready_no_actions() {
		let mut world = ControlFlowPlugin::world();
		let observed = observer_ext::observe_triggers::<Outcome>(&mut world);

		world
			.spawn((AwaitReady::default(), children![EndWith(Outcome::Pass)]))
			.trigger_target(GetOutcome)
			.flush();

		observed.len().xpect_eq(1);
		observed.get_index(0).unwrap().xpect_eq(Outcome::Pass);
	}

	#[sweet::test]
	async fn await_ready_waits_for_actions() {
		let store = Store::default();
		let mut world =
			(MinimalPlugins, AsyncPlugin, ControlFlowPlugin).into_world();

		world
			.spawn((Sequence, children![
				AwaitReady::default(),
				(
					EndWith(Outcome::Pass),
					OnSpawn::observe(move |_: On<GetOutcome>| {
						store.set(true);
					})
				),
				ReadyAction::new(async |_| {
					beet_core::exports::futures_lite::future::yield_now().await;
				}),
			]))
			.trigger_target(GetOutcome)
			.flush();

		// Before async completes, the sequence should not have moved to second child
		store.get().xpect_eq(false);

		AsyncRunner::flush_async_tasks(&mut world).await;

		// After async completes, AwaitReady should pass and sequence continues
		store.get().xpect_eq(true);
	}

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
