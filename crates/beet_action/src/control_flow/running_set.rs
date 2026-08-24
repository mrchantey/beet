//! The one long-running-facets mechanism: a facet is a closure that starts its
//! work, holds it open across a shutdown signal and tears it down, and the set
//! owns its entity's single parked action.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use core::marker::PhantomData;

/// The long-running facets of one entity, and the parked action they share.
///
/// Calling the entity parks a [`Running<Out>`], fires a [`StartRunning<In>`] for
/// any observers, then drives every facet the start selected concurrently under
/// one task. A facet that errors stops the rest gracefully and fails the parked
/// call with the collapsed errors; a start no facet selected fails it loudly (see
/// [`ExcludeRunningErrors`]). Removing that `Running` (an interrupt, a reload, a
/// despawn) signals every live facet, which is the only way a facet is ever
/// stopped: there is no stop action, stopping is signalling.
///
/// A server is the reference facet, so `<Route path="serve" {(HttpServer,
/// TuiServer)}>` is one entity holding one action and two facets. A facet joins
/// through [`RunningSet::add`]; this is never inserted directly.
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
	/// Every facet declared on this entity, in the order they were added.
	facets: Vec<RunningSetFacet<In>>,
	/// One sender per facet the current run started, drained when it stops.
	stop_signals: Vec<OnceValue<()>>,
	/// The parked output, named only by the action this set installs.
	_marker: PhantomData<fn() -> Out>,
}

/// One long-running facet: what it is called in diagnostics, which starts it
/// takes part in, and what it does.
struct RunningSetFacet<In> {
	/// Names this facet in selection failures and error messages.
	label: SmolStr,
	/// Whether a given start includes this facet, ie `--server` naming it.
	select: Box<dyn Fn(&In) -> bool + Send + Sync>,
	/// The facet itself.
	func: RunningSetFn<In>,
}

/// A facet's whole behaviour in one call: it starts the work, holds it open
/// across the shutdown receiver, and tears it down once that receiver resolves.
///
/// The returned future is local (never `Send`): a serve loop is thread-bound, ie
/// the lambda backend holds a tokio `EnterGuard` across its awaits. Shared behind
/// an [`Arc`] so the driver can build its futures outside the world access that
/// selected them, and [`Fn`] so a stopped set can start again.
pub type RunningSetFn<In> = Arc<
	dyn Fn(
			AsyncEntity,
			&In,
			OnceValueRx<()>,
		) -> LocalBoxedFuture<'static, Result>
		+ Send
		+ Sync,
>;

impl<In, Out> Default for RunningSet<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			facets: Vec::new(),
			stop_signals: Vec::new(),
			_marker: PhantomData,
		}
	}
}

impl<In, Out> RunningSet<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	/// Append a facet, get-or-inserting the set on `entity`.
	///
	/// The one way a long-running facet joins an entity's run: called from the
	/// facet component's `on_add` hook, so co-resident facets accumulate rather
	/// than clobber, and the entity ends up with exactly one action.
	pub fn add<Select, Func>(
		entity: &mut EntityCommands,
		label: impl Into<SmolStr>,
		select: Select,
		func: Func,
	) where
		Select: 'static + Send + Sync + Fn(&In) -> bool,
		Func: 'static
			+ Send
			+ Sync
			+ Fn(
				AsyncEntity,
				&In,
				OnceValueRx<()>,
			) -> LocalBoxedFuture<'static, Result>,
	{
		let facet = RunningSetFacet {
			label: label.into(),
			select: Box::new(select),
			func: Arc::new(func),
		};
		entity.queue(move |mut entity: EntityWorldMut| {
			match entity.get_mut::<Self>() {
				Some(mut set) => set.facets.push(facet),
				None => {
					entity.insert(Self {
						facets: vec![facet],
						..default()
					});
				}
			}
		});
	}

	/// The [`Action`] this set installs on its entity: park the call on a
	/// [`Running<Out>`], fan the input out as a [`StartRunning<In>`], then drive
	/// the selected facets.
	pub fn action() -> Action<In, Out> {
		Action::new(
			ActionMeta::of::<Self, In, Out>(),
			|ActionCall {
			     mut commands,
			     caller,
			     input,
			     out_handler,
			 }| {
				// the fan-out and the driver share one input slot, so an observer
				// reading the input sees exactly what the facets will.
				let start = StartRunning::new(caller, input);
				let driven = start.clone();
				// park first: a synchronous `EndRun` from an observer or a facet
				// always lands on a `Running`.
				commands
					.commands
					.entity(caller)
					.insert(Running::new(out_handler))
					.trigger(move |_| start);
				commands
					.entity(caller)
					.run_local(move |entity| drive::<In, Out>(entity, driven));
				Ok(())
			},
		)
	}

	/// The `on_add` hook wiring the stop signals. An associated fn rather than an
	/// inline [`hook_ext::observe`] call, since a hook expression is lowered into
	/// a nested fn that cannot name this component's generics.
	fn observe_stop(
		world: bevy::ecs::world::DeferredWorld,
		cx: bevy::ecs::lifecycle::HookContext,
	) {
		hook_ext::observe(stop_running::<In, Out>)(world, cx);
	}
}

/// Drive one start of an entity's [`RunningSet`] from end to end.
///
/// One task holds every selected facet, so their futures are polled together
/// under a single graceful join rather than detached beyond the run's reach.
async fn drive<In, Out>(entity: AsyncEntity, start: StartRunning<In>) -> Result
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let (input, selected) =
		match select_facets::<In, Out>(&entity, start).await? {
			StartPlan::Aborted => return Ok(()),
			StartPlan::NoneStarted { labels } => {
				return fail_none_started::<Out>(&entity, labels).await;
			}
			StartPlan::Started { input, selected } => (input, selected),
		};

	// the facets own clones of whatever they need, so building their futures
	// borrows the input rather than threading it from one to the next.
	let mut futures = selected
		.into_iter()
		.map(|(func, shutdown)| Some(func(entity.clone(), &input, shutdown)))
		.collect::<Vec<_>>();

	let mut errors = Vec::new();
	while let Some(err) = async_ext::join_all_until_err(&mut futures).await {
		// the first failure ends the run: signal the survivors so each tears its
		// own work down, then keep awaiting them rather than dropping them.
		if errors.is_empty() {
			signal_stop::<In, Out>(&entity).await;
		}
		errors.push(err);
	}
	match errors.is_empty() {
		true => Ok(()),
		false => entity.queue(FailRun::<Out>::new(errors.collapse())).await?,
	}
}

/// What one start resolved to, decided under a single world access so a
/// concurrent stop either precedes the whole start or reaches every facet of it.
enum StartPlan<In> {
	/// Nothing must start: the parked [`Running`] was gone before the driver ran,
	/// ie a same-frame interrupt beat it.
	Aborted,
	/// Every declared facet declined, named here for the failure.
	NoneStarted { labels: Vec<SmolStr> },
	/// The selected facets, each with the receiver it shuts down on.
	Started {
		input: In,
		selected: Vec<(RunningSetFn<In>, OnceValueRx<()>)>,
	},
}

/// Take the start's input, ask each facet whether it takes part, and register a
/// stop signal for every one that does.
async fn select_facets<In, Out>(
	entity: &AsyncEntity,
	start: StartRunning<In>,
) -> Result<StartPlan<In>>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	entity
		.with(move |mut entity| -> Result<StartPlan<In>> {
			if !entity.contains::<Running<Out>>() {
				return StartPlan::Aborted.xok();
			}
			// the driver owns the input; a `StartRunning` observer reads it, never takes it
			let input = start.take()?;
			let mut set =
				entity.get_mut::<RunningSet<In, Out>>().ok_or_else(|| {
					bevyhow!("a RunningSet action outlived its set")
				})?;
			let (senders, selected): (Vec<_>, Vec<_>) = set
				.facets
				.iter()
				.filter(|facet| (facet.select)(&input))
				.map(|facet| {
					let (sender, receiver) = OnceValue::oneshot();
					(sender, (facet.func.clone(), receiver))
				})
				.unzip();
			if selected.is_empty() {
				return StartPlan::NoneStarted {
					labels: set
						.facets
						.iter()
						.map(|facet| facet.label.clone())
						.collect(),
				}
				.xok();
			}
			// registered before any facet is built, so a stop landing between this
			// access and the first poll still reaches every one of them.
			set.stop_signals = senders;
			StartPlan::Started { input, selected }.xok()
		})
		.await
		.flatten()
}

/// Fire and clear every live stop signal, so each started facet tears down.
///
/// A despawned entity has already been drained by [`stop_running`], so a missing
/// set simply means there is nothing left to signal.
async fn signal_stop<In, Out>(entity: &AsyncEntity)
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let signals = entity
		.get_mut::<RunningSet<In, Out>, _>(|mut set| {
			core::mem::take(&mut set.stop_signals)
		})
		.await
		.unwrap_or_default();
	for signal in signals {
		signal.signal(());
	}
}

/// Fail the parked call: every declared facet declined, so nothing holds the run
/// open and the caller would park forever.
async fn fail_none_started<Out>(
	entity: &AsyncEntity,
	labels: Vec<SmolStr>,
) -> Result
where
	Out: 'static + Send + Sync,
{
	if entity
		.get::<ExcludeRunningErrors, _>(|exclude| {
			exclude.contains(RunningError::NONE_STARTED)
		})
		.await
		.unwrap_or(false)
	{
		return Ok(());
	}
	let labels = labels.join(", ");
	entity
		.queue(FailRun::<Out>::new(bevyhow!(
			"declared facets [{labels}] all declined this start"
		)))
		.await?
}

/// Signals every live stop signal when the set's parked [`Running`] is removed.
///
/// Directly in the observer rather than through a queued command: a signal is a
/// world-free value, so a despawn tears down exactly like an interrupt does.
fn stop_running<In, Out>(
	ev: On<Remove, Running<Out>>,
	mut sets: Query<&mut RunningSet<In, Out>>,
) where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let Ok(mut set) = sets.get_mut(ev.event().event_target()) else {
		return;
	};
	for signal in core::mem::take(&mut set.stop_signals) {
		signal.signal(());
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Whether a start includes a facet, boxed so a test can hold facets of
	/// differing selections in one list.
	type Select = Box<dyn Fn(&u32) -> bool + Send + Sync>;
	/// A boxed facet, likewise.
	type Facet = Box<
		dyn Fn(
				AsyncEntity,
				&u32,
				OnceValueRx<()>,
			) -> LocalBoxedFuture<'static, Result>
			+ Send
			+ Sync,
	>;

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ActionPlugin));
		app
	}

	/// A facet logging `{name}-start` as it starts and `{name}-stop` once its
	/// shutdown resolves, so both halves of its lifecycle are observable. It holds
	/// the run open until then, ending with `stop_err` if given.
	fn facet(
		log: Store<Vec<String>>,
		name: &'static str,
		stop_err: Option<&'static str>,
	) -> Facet {
		Box::new(move |_entity, _input, shutdown| {
			log.push(format!("{name}-start"));
			Box::pin(async move {
				shutdown.wait().await;
				log.push(format!("{name}-stop"));
				match stop_err {
					Some(err) => bevybail!("{err}"),
					None => Ok(()),
				}
			})
		})
	}

	/// A facet that fails the moment the driver polls it.
	fn failing_facet(err: &'static str) -> Facet {
		Box::new(move |_entity, _input, _shutdown| {
			Box::pin(async move { bevybail!("{err}") })
		})
	}

	fn always() -> Select { Box::new(|_| true) }
	fn never() -> Select { Box::new(|_| false) }

	/// Spawn an entity holding `facets`, flushing so the set and the action it
	/// installs are both in place before any call.
	fn spawn_set(
		app: &mut App,
		facets: Vec<(&'static str, Select, Facet)>,
	) -> Entity {
		let entity = app.world_mut().spawn_empty().id();
		let mut commands = app.world_mut().commands();
		let mut entity_commands = commands.entity(entity);
		for (label, select, func) in facets {
			RunningSet::<u32, u32>::add(
				&mut entity_commands,
				label,
				select,
				func,
			);
		}
		app.world_mut().flush();
		entity
	}

	/// Call `entity`'s action, parking its outcome in the returned store: it stays
	/// empty for as long as the run holds.
	fn call(
		app: &mut App,
		entity: Entity,
	) -> Store<Option<Result<u32, String>>> {
		let result = Store::<Option<Result<u32, String>>>::default();
		app.world_mut()
			.entity_mut(entity)
			.call_with(
				7u32,
				OutHandler::<u32>::new(move |_, out| {
					result.set(Some(out.map_err(|err| err.to_string())));
					Ok(())
				}),
			)
			.unwrap();
		app.world_mut().flush();
		result
	}

	/// Drive until `log` holds `len` entries, failing fast rather than hanging.
	async fn until_logged(app: &mut App, log: Store<Vec<String>>, len: usize) {
		app_ext::update_until(app, |_| log.len() >= len)
			.await
			.xpect_true();
	}

	/// Every selected facet starts, in the order it was added, and the run parks.
	#[beet_core::test]
	async fn starts_every_facet_in_order() {
		let log = Store::<Vec<String>>::default();
		let mut app = app();
		let entity = spawn_set(&mut app, vec![
			("first", always(), facet(log, "first", None)),
			("second", always(), facet(log, "second", None)),
		]);
		let result = call(&mut app, entity);
		until_logged(&mut app, log, 2).await;
		log.get().xpect_eq(vec!["first-start", "second-start"]);
		// nothing resolved the call, so the run is parked
		result.get().xpect_none();
		app.world()
			.entity(entity)
			.contains::<Running<u32>>()
			.xpect_true();
	}

	/// A facet takes part only in the starts its `select` claims; both facets are
	/// selected in the one pass, so an unlogged one really declined.
	#[beet_core::test]
	async fn select_filters_by_input() {
		let log = Store::<Vec<String>>::default();
		let mut app = app();
		let entity = spawn_set(&mut app, vec![
			(
				"even",
				Box::new(|input: &u32| input % 2 == 0),
				facet(log, "even", None),
			),
			(
				"odd",
				Box::new(|input: &u32| input % 2 == 1),
				facet(log, "odd", None),
			),
		]);
		call(&mut app, entity);
		until_logged(&mut app, log, 1).await;
		log.get().xpect_eq(vec!["odd-start"]);
	}

	/// A start every facet declined holds nothing open, so it fails the call
	/// naming what was declared rather than parking the caller forever.
	#[beet_core::test]
	async fn no_facet_selected_fails_the_call() {
		let log = Store::<Vec<String>>::default();
		let mut app = app();
		let entity = spawn_set(&mut app, vec![
			("http", never(), facet(log, "http", None)),
			("ssh", never(), facet(log, "ssh", None)),
		]);
		let result = call(&mut app, entity);
		app_ext::update_until(&mut app, |_| result.get().is_some())
			.await
			.xpect_true();
		result
			.get()
			.unwrap()
			.unwrap_err()
			.xpect_contains("http")
			.xpect_contains("ssh");
		app.world()
			.entity(entity)
			.contains::<Running<u32>>()
			.xpect_false();
	}

	/// An entity excluding that failure parks silently instead.
	#[beet_core::test]
	async fn excluded_none_started_parks() {
		let log = Store::<Vec<String>>::default();
		let mut app = app();
		let entity = spawn_set(&mut app, vec![(
			"http",
			never(),
			facet(log, "http", None),
		)]);
		app.world_mut()
			.entity_mut(entity)
			.insert(ExcludeRunningErrors(RunningError::NONE_STARTED));
		let result = call(&mut app, entity);
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		result.get().xpect_none();
		app.world()
			.entity(entity)
			.contains::<Running<u32>>()
			.xpect_true();
	}

	/// One facet's error stops the survivors gracefully: their teardown still runs,
	/// and everything that broke reaches the caller.
	#[beet_core::test]
	async fn a_facet_error_stops_the_others() {
		let log = Store::<Vec<String>>::default();
		let mut app = app();
		let entity = spawn_set(&mut app, vec![
			("boom", always(), failing_facet("boom failed")),
			(
				"held",
				always(),
				facet(log, "held", Some("held teardown failed")),
			),
		]);
		let result = call(&mut app, entity);
		app_ext::update_until(&mut app, |_| result.get().is_some())
			.await
			.xpect_true();
		log.get().xpect_eq(vec!["held-start", "held-stop"]);
		result
			.get()
			.unwrap()
			.unwrap_err()
			.xpect_contains("boom failed")
			.xpect_contains("held teardown failed");
	}

	/// Removing the parked `Running` signals every facet, whose own teardown then
	/// runs.
	#[beet_core::test]
	async fn removing_running_stops_every_facet() {
		let log = Store::<Vec<String>>::default();
		let mut app = app();
		let entity = spawn_set(&mut app, vec![
			("a", always(), facet(log, "a", None)),
			("b", always(), facet(log, "b", None)),
		]);
		call(&mut app, entity);
		until_logged(&mut app, log, 2).await;
		app.world_mut().entity_mut(entity).remove::<Running<u32>>();
		app.world_mut().flush();
		until_logged(&mut app, log, 4).await;
		log.get()
			.xpect_eq(vec!["a-start", "b-start", "a-stop", "b-stop"]);
	}

	/// A facet closing a live listener must still tear down when the removal is a
	/// despawn.
	#[beet_core::test]
	async fn despawn_stops_every_facet() {
		let log = Store::<Vec<String>>::default();
		let mut app = app();
		let entity =
			spawn_set(&mut app, vec![("a", always(), facet(log, "a", None))]);
		call(&mut app, entity);
		until_logged(&mut app, log, 1).await;
		app.world_mut().entity_mut(entity).despawn();
		app.world_mut().flush();
		until_logged(&mut app, log, 2).await;
		log.get().xpect_eq(vec!["a-start", "a-stop"]);
	}

	/// A stopped set starts again: the facets are reusable, not consumed by a run.
	#[beet_core::test]
	async fn a_stopped_set_starts_again() {
		let log = Store::<Vec<String>>::default();
		let mut app = app();
		let entity =
			spawn_set(&mut app, vec![("a", always(), facet(log, "a", None))]);
		call(&mut app, entity);
		until_logged(&mut app, log, 1).await;
		app.world_mut().entity_mut(entity).remove::<Running<u32>>();
		app.world_mut().flush();
		until_logged(&mut app, log, 2).await;
		call(&mut app, entity);
		until_logged(&mut app, log, 3).await;
		log.get().xpect_eq(vec!["a-start", "a-stop", "a-start"]);
	}

	/// An interrupt landing before the driver's first world access starts nothing,
	/// so no facet is left holding work open for a run that is already over.
	#[beet_core::test]
	async fn an_interrupt_before_the_driver_starts_nothing() {
		let log = Store::<Vec<String>>::default();
		let mut app = app();
		let entity =
			spawn_set(&mut app, vec![("a", always(), facet(log, "a", None))]);
		call(&mut app, entity);
		app.world_mut().entity_mut(entity).remove::<Running<u32>>();
		app.world_mut().flush();
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		log.get().xpect_eq(Vec::<String>::new());
	}
}
