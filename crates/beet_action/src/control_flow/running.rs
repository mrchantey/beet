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
}

/// Turns an action into a long-running one.
///
/// When called, the [`start_running`] handler stores the [`OutHandler`] on a
/// [`Running`] component and returns without resolving it, so the call stays
/// pending until an [`EndRun`] is queued.
#[derive(Component)]
#[require(RunTimer)]
#[require(Action<In, Out> = start_running::<In, Out>())]
pub struct ContinueRun<In = (), Out = ()>
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

impl<T> EntityCommand<Result> for EndRun<T>
where
	T: 'static + Send + Sync,
{
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
}
