//! The one long-running-facets mechanism: a component contributes what starting
//! it does and what stopping it undoes, and the set owns its entity's single
//! parked action.
use crate::prelude::*;
use beet_core::prelude::*;
use core::marker::PhantomData;

/// The long-running facets of one entity, and the parked action they share.
///
/// Calling the entity parks a [`Running<Out>`], fires a [`StartRunning<In>`] for
/// any observers, then walks the start actions in contribution order with
/// sequence semantics (see [`Sequence`] for the threading): the input moves from
/// entry to entry, so a non-[`Clone`] input needs no copy; an entry that
/// [`Declined`](StartOutcome::Declined) is skipped and the walk goes on; an entry
/// that errors breaks the walk and fails the parked call. Removing that `Running`
/// (an interrupt, a reload, a despawn) walks the stop actions in the same order,
/// so stopping is symmetrical with every other long-running action.
///
/// Servers are the reference contributor, so `<Route path="serve" {(HttpServer,
/// TuiServer)}>` is one entity holding one action and two facets. A facet joins
/// through [`RunningSet::contribute`]; this is never inserted directly.
///
/// The set reports what the walk did with a [`RunningSetStarted`], leaving the
/// meaning of "nothing started" to whichever layer declared the facets.
#[derive(Component)]
#[require(RunTimer)]
#[require(Action<In, Out> = RunningSet::<In, Out>::action())]
#[component(
	on_add = Action::<In, Out>::assert_provider::<Self>,
	on_add = RunningSet::<In, Out>::observe_stop
)]
pub struct RunningSet<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	/// What each facet does to start the run, walked in contribution order.
	on_start: Vec<Action<In, StartOutcome<In>>>,
	/// What each facet undoes when the run ends, walked in the same order.
	on_stop: Vec<Action<(), ()>>,
	/// The parked output, named only by the action this set installs.
	_marker: PhantomData<fn() -> Out>,
}

/// What one [`RunningSet`] start entry did, carrying the input on to the next
/// entry either way.
pub enum StartOutcome<In> {
	/// The entry started: something now holds the run open.
	Started(In),
	/// The entry declined, ie `--server` did not name this server. The walk
	/// continues; a set where every entry declines holds nothing open.
	Declined(In),
}

/// Fired on the entity once a [`RunningSet`] has walked every start entry,
/// reporting what the walk did.
///
/// The set is deliberately policy-free: a consumer that knows a run with no
/// starter is meaningless (a boot whose `--server` selected nothing) observes
/// this and fails the parked call with a [`FailRun`], resolving it in the same
/// breath rather than a frame later.
#[derive(Debug, Clone, EntityEvent)]
pub struct RunningSetStarted {
	/// The entity whose set started.
	pub entity: Entity,
	/// Entries that started the run.
	pub started: usize,
	/// Entries that declined it.
	pub declined: usize,
}

impl<In, Out> Default for RunningSet<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			on_start: Vec::new(),
			on_stop: Vec::new(),
			_marker: PhantomData,
		}
	}
}

impl<In, Out> RunningSet<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	/// Append a facet's start action, and its matching stop action where it has
	/// one, get-or-inserting the set on `entity`.
	///
	/// The one way a long-running facet joins an entity's run: called from the
	/// facet component's `on_add` hook, so co-resident facets accumulate rather
	/// than clobber, and the entity ends up with exactly one action.
	pub fn contribute(
		entity: &mut EntityCommands,
		on_start: Action<In, StartOutcome<In>>,
		on_stop: Option<Action<(), ()>>,
	) {
		entity.queue(move |mut entity: EntityWorldMut| {
			match entity.get_mut::<Self>() {
				Some(mut set) => {
					set.on_start.push(on_start);
					set.on_stop.extend(on_stop);
				}
				None => {
					entity.insert(Self {
						on_start: vec![on_start],
						on_stop: on_stop.into_iter().collect(),
						_marker: PhantomData,
					});
				}
			}
		});
	}

	/// The `on_add` hook wiring the stop walk. An associated fn rather than an
	/// inline [`hook_ext::observe`] call, since a hook expression is lowered into
	/// a nested fn that cannot name this component's generics.
	fn observe_stop(
		world: bevy::ecs::world::DeferredWorld,
		cx: bevy::ecs::lifecycle::HookContext,
	) {
		hook_ext::observe(stop_running::<In, Out>)(world, cx);
	}

	/// The [`Action`] this set installs on its entity: park the call on a
	/// [`Running<Out>`], fan the input out as a [`StartRunning<In>`], then walk
	/// the start entries.
	pub fn action() -> Action<In, Out> {
		Action::new(
			ActionMeta::of::<Self, In, Out>(),
			|ActionCall {
			     mut commands,
			     caller,
			     input,
			     out_handler,
			 }| {
				// the fan-out and the walk share one input slot, so an observer
				// reading the input sees exactly what the entries will.
				let start = StartRunning::new(caller, input);
				let walk = start.clone();
				// park first: a synchronous `EndRun` from an observer or a start
				// entry always lands on a `Running`.
				commands
					.commands
					.entity(caller)
					.insert(Running::new(out_handler))
					.trigger(move |_| start);
				commands
					.entity(caller)
					.run_local(move |entity| walk_start::<In, Out>(entity, walk));
				Ok(())
			},
		)
	}
}

/// Walk the start entries in order, threading the input, then report the tally.
///
/// A failing entry breaks the walk and fails the parked call, so the error
/// resolves the caller rather than dying in this detached task.
async fn walk_start<In, Out>(
	entity: AsyncEntity,
	start: StartRunning<In>,
) -> Result
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let entries = entity
		.get::<RunningSet<In, Out>, _>(|set| set.on_start.clone())
		.await?;
	// the walk owns the input; a `StartRunning` observer must read it, never take it
	let mut input = start.take()?;
	let (mut started, mut declined) = (0, 0);
	for entry in entries {
		match entity.call_detached(entry, input).await {
			Ok(StartOutcome::Started(next)) => {
				started += 1;
				input = next;
			}
			Ok(StartOutcome::Declined(next)) => {
				declined += 1;
				input = next;
			}
			Err(err) => {
				return entity.queue(FailRun::<Out>::new(err)).await?;
			}
		}
	}
	entity
		.trigger(move |entity| RunningSetStarted {
			entity,
			started,
			declined,
		})
		.await
}

/// Walks the set's stop actions when its parked [`Running`] is removed.
///
/// The walk is queued as a world command rather than an entity one: the removal
/// is often a despawn, and a stop that closes a live listener must still run when
/// its entity is already gone.
fn stop_running<In, Out>(
	ev: On<Remove, Running<Out>>,
	sets: Query<&RunningSet<In, Out>>,
	mut commands: Commands,
) where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let entity = ev.event().event_target();
	let Ok(set) = sets.get(entity) else {
		return;
	};
	let entries = set.on_stop.clone();
	commands.queue(move |world: &mut World| -> Result {
		for entry in entries {
			entry.call_world_for(world, entity, (), default())?;
		}
		Ok(())
	});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A start entry recording that it ran, then starting or declining.
	fn entry(
		log: Store<Vec<&'static str>>,
		name: &'static str,
		started: bool,
	) -> Action<u32, StartOutcome<u32>> {
		Action::new_pure(move |cx: ActionContext<u32>| {
			log.push(name);
			match started {
				true => StartOutcome::Started(cx.input),
				false => StartOutcome::Declined(cx.input),
			}
		})
	}

	/// A stop entry recording that it ran.
	fn record(
		log: Store<Vec<&'static str>>,
		name: &'static str,
	) -> Action<(), ()> {
		Action::new_pure(move |_: ActionContext| {
			log.push(name);
		})
	}

	/// Spawn an entity whose set holds `entries`, call it, and drive until the
	/// tally lands.
	async fn drive(
		app: &mut App,
		entries: Vec<(Action<u32, StartOutcome<u32>>, Option<Action<(), ()>>)>,
	) -> (Entity, Store<Option<RunningSetStarted>>) {
		let tally = Store::<Option<RunningSetStarted>>::default();
		let recorder = tally;
		let entity = app.world_mut().spawn_empty().id();
		contribute_all(app, entity, entries);
		app.world_mut().entity_mut(entity).observe_any(
			move |ev: On<RunningSetStarted>| {
				recorder.set(Some(ev.event().clone()))
			},
		);
		app.world_mut()
			.entity_mut(entity)
			.run_async_local(|entity| async move {
				entity.call::<u32, u32>(7).await.ok();
				Ok(())
			});
		app_ext::update_until(app, |_| tally.get().is_some())
			.await
			.xpect_true();
		(entity, tally)
	}

	/// Contribute each start/stop pair to `entity`'s set, then flush so the set
	/// (and the action it installs) is in place before the call.
	fn contribute_all(
		app: &mut App,
		entity: Entity,
		entries: Vec<(Action<u32, StartOutcome<u32>>, Option<Action<(), ()>>)>,
	) {
		let mut commands = app.world_mut().commands();
		let mut entity_commands = commands.entity(entity);
		for (on_start, on_stop) in entries {
			RunningSet::<u32, u32>::contribute(
				&mut entity_commands,
				on_start,
				on_stop,
			);
		}
		app.world_mut().flush();
	}

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ActionPlugin));
		app
	}

	/// Every entry runs in contribution order, a decline is skipped rather than
	/// ending the walk, and the tally separates the two.
	#[beet_core::test]
	async fn walks_every_entry_in_order() {
		let log = Store::<Vec<&'static str>>::default();
		let mut app = app();
		let (entity, tally) = drive(&mut app, vec![
			(entry(log, "first", true), None),
			(entry(log, "declined", false), None),
			(entry(log, "last", true), None),
		])
		.await;
		log.get().xpect_eq(vec!["first", "declined", "last"]);
		let tally = tally.get().unwrap();
		tally.started.xpect_eq(2);
		tally.declined.xpect_eq(1);
		// nothing resolved the call, so the run is parked
		app.world()
			.entity(entity)
			.contains::<Running<u32>>()
			.xpect_true();
	}

	/// Removing the parked `Running` walks the stop entries, in the same order.
	#[beet_core::test]
	async fn stopping_walks_the_stop_entries() {
		let log = Store::<Vec<&'static str>>::default();
		let mut app = app();
		let (entity, _) = drive(&mut app, vec![
			(entry(log, "start-a", true), Some(record(log, "stop-a"))),
			(entry(log, "start-b", true), Some(record(log, "stop-b"))),
		])
		.await;
		log.clear();
		app.world_mut().entity_mut(entity).remove::<Running<u32>>();
		app.world_mut().flush();
		log.get().xpect_eq(vec!["stop-a", "stop-b"]);
	}

	/// A stop still closes what its start opened when the removal is a despawn.
	#[beet_core::test]
	async fn stopping_survives_a_despawn() {
		let log = Store::<Vec<&'static str>>::default();
		let mut app = app();
		let (entity, _) = drive(&mut app, vec![(
			entry(log, "start", true),
			Some(record(log, "stop")),
		)])
		.await;
		log.clear();
		app.world_mut().entity_mut(entity).despawn();
		app.world_mut().flush();
		log.get().xpect_eq(vec!["stop"]);
	}

	/// A failing entry breaks the walk and fails the parked call, so the caller
	/// hears the error rather than parking forever.
	#[beet_core::test]
	async fn a_failing_entry_fails_the_call() {
		let log = Store::<Vec<&'static str>>::default();
		let err = Store::<Option<String>>::default();
		let mut app = app();
		let entity = app.world_mut().spawn_empty().id();
		contribute_all(&mut app, entity, vec![
			(
				Action::new_pure(
					|_: ActionContext<u32>| -> Result<StartOutcome<u32>> {
						bevybail!("no port for you")
					},
				),
				None,
			),
			(entry(log, "never", true), None),
		]);
		app.world_mut()
			.entity_mut(entity)
			.run_async_local(move |entity| async move {
				if let Err(caught) = entity.call::<u32, u32>(7).await {
					err.set(Some(caught.to_string()));
				}
				Ok(())
			});
		app_ext::update_until(&mut app, |_| err.get().is_some())
			.await
			.xpect_true();
		err.get().unwrap().xpect_contains("no port for you");
		// the walk broke, so the entry after the failure never ran
		log.get().xpect_eq(Vec::<&'static str>::new());
		app.world()
			.entity(entity)
			.contains::<Running<u32>>()
			.xpect_false();
	}
}
