//! Action-to-observer fan-out, built on the long-running action machinery.
//!
//! An entity holds at most one [`Action<In, Out>`] per signature, so two actions
//! of the same shape clobber on one entity. [`ActionTrigger`] occupies that slot
//! with a fan-out handler: calling the entity's action goes pending via a
//! [`Running<Out>`] and fires an [`ActionIn`] event that any number of observers
//! can read. One of them resolves the call by queuing an [`EndRun`]; with no
//! observer the call parks on its [`Running`].
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use core::marker::PhantomData;

/// Entity event carrying an action's input to its observers behind a shared
/// handle, so many observers can read it and one can take it.
///
/// Fired by [`action_trigger`] on the caller once a [`Running<Out>`] is in place.
/// The response returns through that [`Running`] plus an [`EndRun`], not through
/// this event, so the non-[`Clone`] `Request`/`Response` bodies are a non-issue.
/// Cheaply cloned via the inner [`Arc`] (no `In: Clone` bound); all clones share
/// one slot, so a [`take`](Self::take) by any is seen by all.
#[derive(EntityEvent)]
pub struct ActionIn<In: 'static + Send + Sync> {
	/// The entity the action was called on.
	pub entity: Entity,
	value: Arc<Mutex<Option<In>>>,
}

impl<In: 'static + Send + Sync> Clone for ActionIn<In> {
	fn clone(&self) -> Self {
		Self {
			entity: self.entity,
			value: self.value.clone(),
		}
	}
}

impl<In: 'static + Send + Sync> ActionIn<In> {
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
			.ok_or_else(|| bevyhow!("ActionIn input already taken"))
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
			.ok_or_else(|| bevyhow!("ActionIn input already taken"))
	}
}

/// Occupies an entity's [`Action<In, Out>`] slot with the fan-out handler, so
/// calling the entity's action fires an [`ActionIn`] to its observers.
///
/// The exchangeable counterpart of [`ContinueRun`]: where `ContinueRun` parks a
/// single call, this fans the call out to any number of observers. A server host
/// carries `ActionTrigger<Request, Response>` so the boot exchange reaches every
/// server that observes [`ActionIn<Request>`].
#[derive(Component)]
#[require(Action<In, Out> = action_trigger::<In, Out>())]
pub struct ActionTrigger<In = (), Out = Outcome>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	_marker: PhantomData<fn() -> (In, Out)>,
}

impl<In, Out> Clone for ActionTrigger<In, Out>
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

impl<In, Out> Default for ActionTrigger<In, Out>
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

/// The [`Action`] behind [`ActionTrigger`]: inserts a [`Running<Out>`] so the
/// call goes pending (the [`start_running`] behaviour), then fires an [`ActionIn`]
/// on the caller for its observers.
///
/// [`Running`] is inserted before [`ActionIn`] fires, so a synchronous [`EndRun`]
/// from an observer always lands on a present [`Running`].
pub fn action_trigger<In, Out>() -> Action<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	Action::new(
		TypeMeta::of::<ActionIn<In>>(),
		|ActionCall {
		     mut commands,
		     caller,
		     input,
		     out_handler,
		 }| {
			commands
				.commands
				.entity(caller)
				.insert(Running::new(out_handler))
				.trigger(move |entity| ActionIn::new(entity, input));
			Ok(())
		},
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn take_and_with_share_one_slot() {
		let ev = ActionIn::new(Entity::PLACEHOLDER, 42u32);
		let clone = ev.clone();
		ev.with(|value| *value).unwrap().xpect_eq(42);
		ev.take().unwrap().xpect_eq(42);
		// the clone shares the slot, so it sees the value already taken
		clone.take().xpect_err();
		clone.with(|value| *value).xpect_err();
	}

	#[beet_core::test]
	async fn fans_out_and_resolves() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(ActionTrigger::<(), Outcome>::default()).id();
		// an observer resolves the pending call when the input fans out to it
		world.entity_mut(entity).observe_any(
			|ev: On<ActionIn<()>>, mut commands: Commands| {
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
	async fn parks_without_observer() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(ActionTrigger::<(), Outcome>::default()).id();

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

		// no observer took the input: the call parks on its Running
		world.get::<Running<Outcome>>(entity).xpect_some();
		store.get().xpect_none();
	}
}
