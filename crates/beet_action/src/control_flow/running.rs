//! Long-running action support and action-to-observer fan-out.
//!
//! In the async action model a handler resolves its [`OutHandler`] to
//! complete a call. A *long-running* action instead defers completion: the
//! [`ContinueRun`] action stores the [`OutHandler`] in a [`Running`] component
//! and returns without resolving it. Some later system or event ends the run
//! by queuing an [`EndRun`] command, which removes [`Running`] and resolves
//! the stored handler with a value.
//!
//! Calling a [`ContinueRun`] also fires an [`StartRunning`] event carrying the
//! input, so any number of observers can react to the parked call. One of them
//! resolves it by queuing an [`EndRun`]; with no observer the call simply parks
//! on its [`Running`]. This is how a boot notification reaches a bystander that
//! wants the request but holds nothing open, ie a thread kicking itself off.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::event::SetEntityEventTarget;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use bevy::reflect::GetTypeRegistration;
use bevy::reflect::Typed;
use core::marker::PhantomData;

/// Marks an action as actively running, holding the deferred [`OutHandler`]
/// used to complete the original call.
///
/// Inserted by [`ContinueRun::start_running`] (the action behind [`ContinueRun`]) and
/// removed by [`EndRun`]. Stored as
/// [`SparseSet`](bevy::ecs::component::StorageType::SparseSet) since it is
/// frequently added and removed. It is a call in flight, so it registers with
/// the entity's [`ActionCalls`] and an [`InterruptRun`] above it resolves it
/// whatever `T` is.
#[derive(Component)]
#[component(storage = "SparseSet", on_add = ActionCalls::register_running::<T>)]
#[require(RunTimer, ActionCalls)]
pub struct Running<T = Outcome>(OutHandler<T>)
where
	T: 'static + Send + Sync;

impl<T> Running<T>
where
	T: 'static + Send + Sync,
{
	/// Wrap the deferred [`OutHandler`].
	pub fn new(out_handler: OutHandler<T>) -> Self { Self(out_handler) }

	/// Resolve the deferred handler with `value`, completing the call.
	///
	/// # Errors
	/// Propagates any error from the [`OutHandler`].
	pub fn end(self, world: &mut World, value: T) -> Result {
		self.0.call_world(world, Ok(value))
	}

	/// Resolve the deferred handler with `error`, completing the call as failed.
	///
	/// # Errors
	/// Propagates any error from the [`OutHandler`].
	pub fn fail(self, world: &mut World, error: BevyError) -> Result {
		self.0.call_world(world, Err(error))
	}

	/// Resolve the deferred handler with [`ControlFlowError::Interrupted`],
	/// completing the call as interrupted.
	///
	/// # Errors
	/// Propagates any error from the [`OutHandler`].
	pub fn interrupt(self, world: &mut World) -> Result {
		self.fail(world, ControlFlowError::Interrupted.into())
	}
}

/// Errors produced when an action call is ended by control flow rather than
/// completing normally.
#[derive(Debug, thiserror::Error)]
pub enum ControlFlowError {
	/// An action call was stopped before it could resolve: an [`InterruptRun`]
	/// above it, a sibling failing or winning a race, its [`Running`] removed,
	/// or its entity despawned.
	#[error("the running action was interrupted")]
	Interrupted,
}

impl ControlFlowError {
	/// Whether `err` is an interruption, so a caller can treat a run ended by
	/// control flow as neither a result nor a fault.
	pub fn is_interrupted(err: &BevyError) -> bool {
		matches!(err.downcast_ref::<Self>(), Some(Self::Interrupted))
	}
}

/// Entity event carrying an action's input to its observers behind a shared
/// handle, so many observers can read it and one can take it.
///
/// Fired by [`ContinueRun::start_running`] on the caller once a [`Running<Out>`] is in place.
/// The response returns through that [`Running`] plus an [`EndRun`], not through
/// this event, so the non-[`Clone`] `Request`/`Response` bodies are a non-issue.
/// Cheaply cloned via the inner [`Arc`] (no `In: Clone` bound); all clones share
/// one slot, so a [`take`](Self::take) by any is seen by all.
///
/// Delivery is a [`ScopedTrigger`]: a caller declaring
/// [`SweepDescendants<StartRunning<In>>`](SweepDescendants) sweeps the event
/// over its subtree, deepest first and itself last, so a descendant start verb
/// (`CallOnStart`) observes its own entity exactly as `CallOnReady` observes
/// its own `Ready`. A bare caller fires on itself alone.
// `TypePath` so that declaration is itself reflectable, ie registering
// `SweepDescendants<StartRunning<Request>>` for markup.
#[derive(TypePath)]
pub struct StartRunning<In: 'static + Send + Sync> {
	/// The entity this instance is firing on: the caller, or under a sweep the
	/// swept entity.
	pub entity: Entity,
	value: Arc<Mutex<Option<In>>>,
}

impl<In: 'static + Send + Sync> Event for StartRunning<In> {
	type Trigger<'a> = ScopedTrigger<Self>;
}

impl<In: 'static + Send + Sync> EntityEvent for StartRunning<In> {
	fn event_target(&self) -> Entity { self.entity }
}

impl<In: 'static + Send + Sync> SetEntityEventTarget for StartRunning<In> {
	fn set_event_target(&mut self, entity: Entity) { self.entity = entity; }
}

impl<In: 'static + Send + Sync> Clone for StartRunning<In> {
	fn clone(&self) -> Self {
		Self {
			entity: self.entity,
			value: self.value.clone(),
		}
	}
}

impl<In: 'static + Send + Sync> StartRunning<In> {
	/// Wrap `input` in a shared slot targeting `entity`.
	pub fn new(entity: Entity, input: In) -> Self {
		Self {
			entity,
			value: Arc::new(Mutex::new(Some(input))),
		}
	}

	/// Read the input without consuming it.
	///
	/// # Errors
	/// Errors if the input has already been [`take`](Self::take)n. A [`Mutex`]
	/// guard cannot lend `&In` past its own lifetime, hence the closure form.
	pub fn with<O>(&self, func: impl FnOnce(&In) -> O) -> Result<O> {
		self.value
			.lock()
			.unwrap()
			.as_ref()
			.map(func)
			.ok_or_else(|| bevyhow!("StartRunning input already taken"))
	}

	/// Take ownership of the input.
	///
	/// # Errors
	/// Errors if the input has already been taken.
	pub fn take(&self) -> Result<In> {
		self.value
			.lock()
			.unwrap()
			.take()
			.ok_or_else(|| bevyhow!("StartRunning input already taken"))
	}
}

/// Turns an action into a long-running, fan-out one: the single "park and emit"
/// primitive.
///
/// When called, the [`ContinueRun::start_running`] handler stores the [`OutHandler`] on a
/// [`Running`] component (so the call stays pending until an [`EndRun`] is
/// queued) and fires an [`StartRunning`] carrying the input to any observers. A
/// behaviour-tree action parks with no observer; an entity whose long-running
/// work is a set of facets uses [`RunningSet`] instead, which parks the same way
/// and additionally drives them.
///
/// Never colocate an explicit `Action<In, Out>` (eg a handler, or a `Router`)
/// with a `ContinueRun<In, Out>`: explicit-beats-required would silently replace
/// the parking action and every dependent (eg a server's boot) goes inert. The
/// provider-integrity guard ([`Action::assert_provider`]) raises instead; move
/// the explicit action to its own entity.
#[derive(Component)]
#[require(RunTimer)]
#[require(Action<In, Out> = ContinueRun::<In, Out>::start_running())]
#[component(on_add = Action::<In, Out>::assert_provider::<Self>)]
pub struct ContinueRun<In = (), Out = Outcome>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	_marker: PhantomData<fn() -> (In, Out)>,
}

impl<In, Out> Clone for ContinueRun<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	fn clone(&self) -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<In, Out> Default for ContinueRun<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl ContinueRun {
	/// Create a default `ContinueRun<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

impl<In, Out> ContinueRun<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	/// The [`Action`] backing [`ContinueRun`]: stores the [`OutHandler`] in a
	/// [`Running`] component (leaving the call pending) and fires an [`StartRunning`]
	/// with the input for any observers.
	pub fn start_running() -> Action<In, Out> {
		Action::new(
			ActionMeta::of::<ContinueRun<In, Out>, In, Out>(),
			|ActionCall {
			     mut commands,
			     caller,
			     input,
			     out_handler,
			 }| {
				// park the call on a `Running<Out>`, then fan the input out to any
				// `StartRunning<In>` observers; `Running` is inserted first so a
				// synchronous `EndRun` from an observer always lands on it.
				commands
					.commands
					.entity(caller)
					.insert(Running::new(out_handler))
					.trigger(move |entity| StartRunning::new(entity, input));
				Ok(())
			},
		)
	}
}

/// Ends a [`Running`] action, resolving its deferred [`OutHandler`].
///
/// Queue on an entity to remove its [`Running<T>`] and complete the original
/// call with the wrapped value.
pub struct EndRun<T = Outcome>(pub T)
where
	T: 'static + Send + Sync;

impl<T> EntityCommand for EndRun<T>
where
	T: 'static + Send + Sync,
{
	type Out = Result;

	fn apply(self, mut entity: EntityWorldMut) -> Result {
		let running = entity.take::<Running<T>>().ok_or_else(|| {
			bevyhow!(
				"EndRun expected a Running<{}> component",
				TypeName::of::<T>()
			)
		})?;
		running.end(entity.into_world_mut(), self.0)
	}
}

/// Fails a [`Running`] action, resolving its deferred [`OutHandler`] with an
/// error: the [`EndRun`] counterpart for a run that could not start, or one a
/// facet aborted.
///
/// Queue on an entity to remove its [`Running<T>`] and fail the original call.
/// With no `Running` (the call already resolved) the error is returned instead,
/// so it surfaces through the caller's own error path rather than vanishing.
pub struct FailRun<T = Outcome> {
	error: BevyError,
	_marker: PhantomData<fn() -> T>,
}

impl<T> FailRun<T>
where
	T: 'static + Send + Sync,
{
	/// Fail the entity's `Running<T>` with `error`.
	pub fn new(error: impl Into<BevyError>) -> Self {
		Self {
			error: error.into(),
			_marker: PhantomData,
		}
	}
}

impl<T> EntityCommand for FailRun<T>
where
	T: 'static + Send + Sync,
{
	type Out = Result;

	fn apply(self, mut entity: EntityWorldMut) -> Result {
		match entity.take::<Running<T>>() {
			Some(running) => running.fail(entity.into_world_mut(), self.error),
			None => Err(self.error),
		}
	}
}

/// Prevents [`InterruptRun`] from interrupting this entity's actions, its
/// parked [`Running`] and its async calls alike.
///
/// Interruption only ever descends from an ancestor, so a direct
/// [`EndRun`] on the entity itself still resolves it.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct NoInterrupt;

/// Asks everything beneath an entity to stop, whatever kind of action it is:
/// every descendant's [`ActionCalls`], each async call in flight and each
/// parked [`Running`] whatever its output, their callers resolving as
/// [`ControlFlowError::Interrupted`].
///
/// Descendants carrying [`NoInterrupt`] are skipped, and the target entity
/// itself is left alone, being the one interrupting or only just started.
/// The descendants' other tasks ([`EntityTasks`], ie a server's connections)
/// keep running; [`all`](Self::all) cancels those too, as [`CancelTasks`]
/// alone does, [`NoInterrupt`] guarding actions only.
///
/// Queue on an entity whose subtree should stop, ie before re-running a
/// parent:
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// let mut world = AsyncPlugin::world();
/// let parent = world.spawn(children![ContinueRun::<(), Outcome>::default()]).id();
/// world.commands().entity(parent).queue(InterruptRun::new());
/// world.flush();
/// ```
#[derive(Debug, Default, Clone)]
pub struct InterruptRun {
	/// Whether the descendants' other tasks are cancelled too.
	tasks: bool,
}

impl InterruptRun {
	/// Interrupt the actions beneath the entity, leaving its other tasks.
	pub fn new() -> Self { Self::default() }

	/// Interrupt the actions beneath the entity and cancel its other tasks.
	pub fn all() -> Self { Self { tasks: true } }
}

impl EntityCommand for InterruptRun {
	type Out = Result;

	fn apply(self, entity: EntityWorldMut) -> Result {
		let target = entity.id();
		let world = entity.into_world_mut();
		let interruptible = world.with_state::<(
			Query<(), (With<ActionCalls>, Without<NoInterrupt>)>,
			Query<&Children>,
		), _>(|(calls, children)| {
			children
				.iter_descendants(target)
				.filter(|child| calls.contains(*child))
				.collect::<Vec<_>>()
		});
		for child in interruptible {
			ActionCalls::interrupt(world, child)?;
		}
		if self.tasks
			&& let Ok(entity) = world.get_entity_mut(target)
		{
			CancelTasks.apply(entity);
		}
		Ok(())
	}
}

/// Registers the long-running action lifecycle for an `In`/`Out` pair:
/// [`EndInDuration`] reflection and its tick system, plus the [`RunTimer`]
/// reset observers keyed on [`Running<Out>`].
///
/// [`tick_run_timers`] and [`RunTimer`] itself are registered once by
/// [`ActionPlugin`] since they are not generic.
pub(crate) fn running_plugin<In, Out>(app: &mut App)
where
	In: 'static + Send + Sync + TypePath,
	Out: 'static
		+ Send
		+ Sync
		+ Clone
		+ Default
		+ FromReflect
		+ Typed
		+ GetTypeRegistration,
{
	use bevy::reflect::std_traits::ReflectDefault;
	app.register_type::<EndInDuration<In, Out>>()
		// a markup `<EndInDuration duration="1s"/>` only sets `duration`, so the
		// reflect path builds the rest from `Default`; register `ReflectDefault` for
		// the concrete type (the derive can't carry `#[reflect(Default)]`, since not
		// every `Out` is `Default`).
		.register_type_data::<EndInDuration<In, Out>, ReflectDefault>()
		.add_systems(Update, end_in_duration::<In, Out>.after(tick_run_timers))
		.add_observer(reset_run_time_started::<Out>)
		.add_observer(reset_run_timer_stopped::<Out>);
}

/// Tracks elapsed time since an action last started and last ended.
///
/// Both timers tick continuously, even when the action is not [`Running`],
/// enabling patterns like "run if inactive for duration".
#[derive(Default, Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RunTimer {
	/// Time since [`Running`] was last added.
	pub last_run: Stopwatch,
	/// Time since [`Running`] was last removed.
	pub last_end: Stopwatch,
}

/// Ticks all [`RunTimer`] components.
pub(crate) fn tick_run_timers(
	time: When<Res<Time>>,
	mut timers: Populated<&mut RunTimer>,
) {
	for mut timer in timers.iter_mut() {
		timer.last_run.tick(time.delta());
		timer.last_end.tick(time.delta());
	}
}

/// Resets `last_run` when [`Running`] is added.
pub(crate) fn reset_run_time_started<T>(
	ev: On<Add<Running<T>>>,
	mut query: Populated<&mut RunTimer>,
) -> Result
where
	T: 'static + Send + Sync,
{
	query.get_mut(ev.event().event_target())?.last_run.reset();
	Ok(())
}

/// Resets `last_end` when [`Running`] is removed.
pub(crate) fn reset_run_timer_stopped<T>(
	ev: On<Remove<Running<T>>>,
	mut query: Populated<&mut RunTimer>,
) -> Result
where
	T: 'static + Send + Sync,
{
	query.get_mut(ev.event().event_target())?.last_end.reset();
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::actions::test_support;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn take_and_with_share_one_slot() {
		let ev = StartRunning::new(Entity::PLACEHOLDER, 42u32);
		let clone = ev.clone();
		ev.with(|value| *value).unwrap().xpect_eq(42);
		ev.take().unwrap().xpect_eq(42);
		// the clone shares the slot, so it sees the value already taken
		clone.take().xpect_err();
		clone.with(|value| *value).xpect_err();
	}

	/// Colocating an explicit same-pair action with a [`ContinueRun`] silently
	/// replaces its parking action (explicit-beats-required); the
	/// provider-integrity guard raises instead of leaving a dead server.
	#[beet_core::test]
	#[should_panic = "lost its action slot"]
	fn clobbered_parking_action_raises() {
		World::new().spawn((
			ContinueRun::<(), Outcome>::default(),
			Action::<(), Outcome>::new_pure(|_: ActionContext| Outcome::PASS),
		));
	}

	#[beet_core::test]
	async fn fans_out_and_resolves() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(ContinueRun::<(), Outcome>::default()).id();
		// an observer resolves the pending call when the input fans out to it
		world.entity_mut(entity).observe_any(
			|ev: On<StartRunning<()>>, mut commands: Commands| {
				commands.entity(ev.entity).queue(EndRun(Outcome::PASS));
			},
		);

		let store = Store::<Option<Outcome>>::default();
		let captured = store.clone();
		world
			.entity_mut(entity)
			.call_with(
				(),
				OutHandler::<Outcome>::new(move |_, result| {
					captured.set(Some(result?));
					Ok(())
				}),
			)
			.unwrap();
		world.flush();

		world.get::<Running<Outcome>>(entity).xpect_none();
		store.get().xpect_eq(Some(Outcome::PASS));
	}

	#[beet_core::test]
	async fn defers_until_end_run() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(ContinueRun::<(), Outcome>::default()).id();

		let store = Store::<Option<Outcome>>::default();
		world
			.entity_mut(entity)
			.call_with(
				(),
				OutHandler::<Outcome>::new(move |_, result| {
					store.set(Some(result?));
					Ok(())
				}),
			)
			.unwrap();

		// the call is pending: Running holds the handler, store is unset
		world.get::<Running<Outcome>>(entity).xpect_some();
		store.get().xpect_none();

		world.commands().entity(entity).queue(EndRun(Outcome::PASS));
		world.flush();

		world.get::<Running<Outcome>>(entity).xpect_none();
		store.get().xpect_eq(Some(Outcome::PASS));
	}

	fn spawn_running_child(
		world: &mut World,
		store: Store<Option<bool>>,
	) -> (Entity, Entity) {
		let child = world.spawn(ContinueRun::<(), Outcome>::default()).id();
		let parent = world.spawn(children![]).add_child(child).id();
		world
			.entity_mut(child)
			.call_with(
				(),
				OutHandler::<Outcome>::new(move |_, result| {
					store.set(Some(result.is_err()));
					Ok(())
				}),
			)
			.unwrap();
		world.get::<Running<Outcome>>(child).xpect_some();
		(parent, child)
	}

	#[beet_core::test]
	async fn interrupts_running_descendants() {
		let mut world = AsyncPlugin::world();
		let store = Store::<Option<bool>>::default();
		let (parent, child) = spawn_running_child(&mut world, store.clone());

		world.commands().entity(parent).queue(InterruptRun::new());
		world.flush();

		world.get::<Running<Outcome>>(child).xpect_none();
		store.get().xpect_eq(Some(true));
	}

	/// What landed in a [`Subtree`] once it settled.
	#[derive(Debug, PartialEq)]
	struct Landed {
		action: bool,
		guarded: bool,
		task: bool,
	}

	/// A parent over an async action in flight, a second carrying
	/// [`NoInterrupt`], and an entity task standing in for a server's
	/// connection, each setting its store once it is done.
	struct Subtree {
		parent: Entity,
		action: Store<bool>,
		guarded: Store<bool>,
		task: Store<bool>,
	}

	impl Subtree {
		fn spawn(world: &mut World) -> Self {
			let (action, guarded, task) = default();
			let parent = world
				.spawn(children![
					test_support::sleep_then_set(30, Store::clone(&action)),
					(
						NoInterrupt,
						test_support::sleep_then_set(
							30,
							Store::clone(&guarded)
						)
					),
				])
				.id();
			let callees =
				world.entity(parent).get::<Children>().unwrap().to_vec();
			for callee in callees {
				world
					.commands()
					.entity(callee)
					.call((), OutHandler::<Outcome>::default());
			}
			let task_store = Store::clone(&task);
			world.spawn(ChildOf(parent)).run_async(move |_| async move {
				time_ext::sleep_millis(30).await;
				task_store.set(true);
			});
			world.flush();
			Self {
				parent,
				action,
				guarded,
				task,
			}
		}

		/// Stop the subtree with `command`, then report what still landed.
		async fn stop(
			self,
			world: &mut World,
			command: impl EntityCommand<Out = Result>,
		) -> Landed {
			command.apply(world.entity_mut(self.parent)).unwrap();
			test_support::settle(world, 80).await;
			Landed {
				action: self.action.get(),
				guarded: self.guarded.get(),
				task: self.task.get(),
			}
		}
	}

	#[beet_core::test]
	async fn interrupt_stops_actions_and_leaves_tasks() {
		let mut world = AsyncPlugin::world();
		Subtree::spawn(&mut world)
			.stop(&mut world, InterruptRun::new())
			.await
			.xpect_eq(Landed {
				action: false,
				guarded: true,
				task: true,
			});
	}

	#[beet_core::test]
	async fn cancel_tasks_leaves_actions() {
		let mut world = AsyncPlugin::world();
		Subtree::spawn(&mut world)
			.stop(&mut world, |entity: EntityWorldMut| -> Result {
				CancelTasks.apply(entity);
				Ok(())
			})
			.await
			.xpect_eq(Landed {
				action: true,
				guarded: true,
				task: false,
			});
	}

	#[beet_core::test]
	async fn interrupt_all_stops_both() {
		let mut world = AsyncPlugin::world();
		Subtree::spawn(&mut world)
			.stop(&mut world, InterruptRun::all())
			.await
			.xpect_eq(Landed {
				action: false,
				guarded: true,
				task: false,
			});
	}

	/// An interrupted async call resolves its awaiting caller as interrupted.
	#[beet_core::test]
	async fn an_interrupted_caller_reads_interrupted() {
		let mut world = AsyncPlugin::world();
		let callee = world
			.spawn(test_support::sleep_then_set(30, default()))
			.id();
		let parent = world.spawn_empty().add_child(callee).id();
		world
			.run_async_local_then(move |world| async move {
				world
					.run_async_local(move |world| async move {
						time_ext::sleep_millis(5).await;
						world.entity(parent).queue(InterruptRun::new()).await?
					})
					.await;
				world.entity(callee).call::<(), Outcome>(()).await
			})
			.await
			.unwrap_err()
			.xmap(|err| ControlFlowError::is_interrupted(&err))
			.xpect_true();
	}

	#[beet_core::test]
	async fn no_interrupt_is_skipped() {
		let mut world = AsyncPlugin::world();
		let store = Store::<Option<bool>>::default();
		let (parent, child) = spawn_running_child(&mut world, store.clone());
		world.entity_mut(child).insert(NoInterrupt);

		world.commands().entity(parent).queue(InterruptRun::new());
		world.flush();

		world.get::<Running<Outcome>>(child).xpect_some();
		store.get().xpect_none();
	}
}
