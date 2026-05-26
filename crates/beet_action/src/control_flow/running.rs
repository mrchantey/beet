//! Long-running action support.
//!
//! In the async action model a handler resolves its [`OutHandler`] to
//! complete a call. A *long-running* action instead defers completion: the
//! [`ContinueRun`] action stores the [`OutHandler`] in a [`Running`] component
//! and returns without resolving it. Some later system or event ends the run
//! by queuing an [`EndRun`] command, which removes [`Running`] and resolves
//! the stored handler with a value.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::GetTypeRegistration;
use bevy::reflect::Typed;
use std::marker::PhantomData;

/// Marks an action as actively running, holding the deferred [`OutHandler`]
/// used to complete the original call.
///
/// Inserted by [`start_running`] (the action behind [`ContinueRun`]) and
/// removed by [`EndRun`]. Stored as
/// [`SparseSet`](bevy::ecs::component::StorageType::SparseSet) since it is
/// frequently added and removed.
#[derive(Component)]
#[component(storage = "SparseSet")]
#[require(RunTimer)]
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

	/// Resolve the deferred handler with [`ControlFlowError::Interrupted`],
	/// completing the call as interrupted.
	///
	/// # Errors
	/// Propagates any error from the [`OutHandler`].
	pub fn interrupt(self, world: &mut World) -> Result {
		self.0
			.call_world(world, Err(ControlFlowError::Interrupted.into()))
	}
}

/// Errors produced when a [`Running`] action is ended by control flow rather
/// than completing normally.
#[derive(Debug, thiserror::Error)]
pub enum ControlFlowError {
	/// A running action was interrupted before it could resolve, ie a sibling
	/// failed or a parent was re-run.
	#[error("the running action was interrupted")]
	Interrupted,
}

/// Turns an action into a long-running one.
///
/// When called, the [`start_running`] handler stores the [`OutHandler`] on a
/// [`Running`] component and returns without resolving it, so the call stays
/// pending until an [`EndRun`] is queued.
#[derive(Component)]
#[require(RunTimer)]
#[require(Action<In, Out> = start_running::<In, Out>())]
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

/// The [`Action`] backing [`ContinueRun`]: stores the [`OutHandler`] in a
/// [`Running`] component and returns, leaving the call pending.
pub fn start_running<In, Out>() -> Action<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	Action::new(
		TypeMeta::of::<ContinueRun<In, Out>>(),
		|ActionCall {
		     mut commands,
		     caller,
		     input,
		     out_handler,
		 }| {
			let _ = input;
			commands
				.commands
				.entity(caller)
				.insert(Running::new(out_handler));
			Ok(())
		},
	)
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
				std::any::type_name::<T>()
			)
		})?;
		running.end(entity.into_world_mut(), self.0)
	}
}


/// Prevents [`InterruptRun`] from interrupting this action.
///
/// Interruption only ever descends from an ancestor, so a direct
/// [`EndRun`]/[`InterruptRun`] on the entity itself still resolves it.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct NoInterrupt;

/// Interrupts every [`Running<T>`] descendant of an entity, resolving each
/// with [`ControlFlowError::Interrupted`].
///
/// Descendants carrying [`NoInterrupt`] are skipped, and the target entity
/// itself is left alone (it has typically only just started).
///
/// Queue on an entity whose subtree should be cancelled, ie before re-running
/// a parent or when a racing sibling has resolved first.
pub struct InterruptRun<T = Outcome>(PhantomData<fn() -> T>)
where
	T: 'static + Send + Sync;

impl<T> Default for InterruptRun<T>
where
	T: 'static + Send + Sync,
{
	fn default() -> Self { Self(PhantomData) }
}

impl<T> InterruptRun<T>
where
	T: 'static + Send + Sync,
{
	/// Create an [`InterruptRun`] for `Running<T>` descendants.
	pub fn new() -> Self { Self::default() }
}

impl<T> EntityCommand for InterruptRun<T>
where
	T: 'static + Send + Sync,
{
	type Out = Result;

	fn apply(self, entity: EntityWorldMut) -> Result {
		let target = entity.id();
		let world = entity.into_world_mut();
		let interruptible = world.with_state::<(
			Query<(), (With<Running<T>>, Without<NoInterrupt>)>,
			Query<&Children>,
		), _>(|(running, children)| {
			children
				.iter_descendants(target)
				.filter(|child| running.contains(*child))
				.collect::<Vec<_>>()
		});
		for child in interruptible {
			if let Some(running) = world.entity_mut(child).take::<Running<T>>()
			{
				running.interrupt(world)?;
			}
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
pub fn running_plugin<In, Out>(app: &mut App)
where
	In: 'static + Send + Sync + TypePath,
	Out: 'static
		+ Send
		+ Sync
		+ Clone
		+ FromReflect
		+ Typed
		+ GetTypeRegistration,
{
	app.register_type::<EndInDuration<In, Out>>()
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
	ev: On<Add, Running<T>>,
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
	ev: On<Remove, Running<T>>,
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
	use crate::prelude::*;
	use beet_core::prelude::*;

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

		world
			.commands()
			.entity(parent)
			.queue(InterruptRun::<Outcome>::new());
		world.flush();

		world.get::<Running<Outcome>>(child).xpect_none();
		store.get().xpect_eq(Some(true));
	}

	#[beet_core::test]
	async fn no_interrupt_is_skipped() {
		let mut world = AsyncPlugin::world();
		let store = Store::<Option<bool>>::default();
		let (parent, child) = spawn_running_child(&mut world, store.clone());
		world.entity_mut(child).insert(NoInterrupt);

		world
			.commands()
			.entity(parent)
			.queue(InterruptRun::<Outcome>::new());
		world.flush();

		world.get::<Running<Outcome>>(child).xpect_some();
		store.get().xpect_none();
	}
}
