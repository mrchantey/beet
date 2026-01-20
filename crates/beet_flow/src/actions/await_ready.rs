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
	num_ready: u32,
	/// The number of descendants with a [`ReadyAction`] component.
	num_actions: u32,
	/// Entities we're waiting for Ready signals from.
	pending: HashSet<Entity>,
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
	// find the agent of this action to scope the search to this exchange only
	let agent = agents.entity(target);

	let entities: HashSet<Entity> = children
		.iter_descendants(agent)
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

		// observe Ready on the agent where events bubble to
		commands.entity(agent).observe(
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

	#[beet_core::test]
	async fn await_ready_waits_for_actions() {
		let store = Store::default();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, ControlFlowPlugin));

		app.world_mut()
			.spawn((Sequence, children![
				AwaitReady::default(),
				(
					EndWith(Outcome::Pass),
					OnSpawn::observe(
						move |_: On<GetOutcome>, mut commands: Commands| {
							store.set(true);
							commands.write_message(AppExit::Success);
						}
					)
				),
				ReadyAction::run(async |_| {
					beet_core::exports::futures_lite::future::yield_now().await;
				}),
			]))
			.trigger_target(GetOutcome)
			.flush();

		// Before async completes, the sequence should not have moved to second child
		store.get().xpect_eq(false);

		app.run_async().await;

		// After async completes, AwaitReady should pass and sequence continues
		store.get().xpect_eq(true);
	}
}
