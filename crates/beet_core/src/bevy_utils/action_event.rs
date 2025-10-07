use crate::prelude::*;
use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::event::Trigger;
use bevy::ecs::event::trigger_entity_internal;
use bevy::ecs::observer::CachedObservers;
use bevy::ecs::observer::TriggerContext;
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::traversal::Traversal;
use std::fmt;
use std::marker::PhantomData;


pub trait ActionEvent:
	for<'a> Event<Trigger<'a> = ActionTrigger<false, Self, &'static ChildOf>>
{
}
impl<T> ActionEvent for T where
	for<'a> T: Event<Trigger<'a> = ActionTrigger<false, T, &'static ChildOf>>
{
}


#[extend::ext(name=OnActionEventExt)]
pub impl<'w, 't, T> On<'w, 't, T>
where
	T: ActionEvent,
{
	fn event_target(&self) -> Entity { self.trigger().event_target() }
	fn original_event_target(&self) -> Entity {
		self.trigger().original_event_target()
	}
}
#[extend::ext(name=WorldActionEventExt)]
pub impl World {
	fn trigger_action<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = ActionTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Send + Sync + Traversal<E>,
	>(
		&mut self,
		ev: &mut E,
		trigger: &mut E::Trigger<'a>,
		caller: MaybeLocation,
	) -> &mut Self {
		let event_key = self.register_event_key::<E>();
		{
			// SAFETY: event_key was just registered and matches `event`
			unsafe {
				DeferredWorld::from(&mut *self)
					.trigger_raw(event_key, ev, trigger, caller);
			}
		}
		self.flush();
		self
	}
}


#[extend::ext(name=EntityCommandsActionEventExt)]
pub impl EntityCommands<'_> {
	fn trigger_action<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = ActionTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Send + Sync + Traversal<E>,
	>(
		&mut self,
		mut ev: E,
	) -> &mut Self {
		let caller = MaybeLocation::caller();
		let mut trigger = ActionTrigger::new(self.id());
		self.queue(move |mut entity: EntityWorldMut| {
			entity.world_scope(move |world| {
				world.trigger_action(&mut ev, &mut trigger, caller);
			});
		});
		self
	}

	/// An [`EntityCommand`] that creates an [`Observer`](crate::observer::Observer)
	/// watching for an [`EntityEvent`] of type `E` whose [`EntityEvent::event_target`]
	/// targets this entity.
	// we need this because `observe` is restricted to [`EntityEvent`]
	#[track_caller]
	fn observe_any<E: Event, B: Bundle, M>(
		&mut self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> &mut Self {
		self.queue(move |mut entity: EntityWorldMut| {
			entity.observe_any(observer);
		});
		self
	}
}

#[extend::ext(name=EntityWorldMutActionEventExt)]
pub impl EntityWorldMut<'_> {
	fn trigger_action<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = ActionTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Send + Sync + Traversal<E>,
	>(
		&mut self,
		mut ev: E,
	) -> &mut Self {
		let caller = MaybeLocation::caller();
		let mut trigger = ActionTrigger::<AUTO_PROPAGATE, E, T>::new(self.id());
		self.world_scope(move |world| {
			world.trigger_action(&mut ev, &mut trigger, caller);
		});
		self
	}

	/// Call [`World::flush`]
	fn flush(&mut self) -> &mut Self {
		self.world_scope(|world| {
			world.flush();
		});
		self
	}
	/// Creates an [`Observer`] watching for an [`EntityEvent`] of type `E` whose [`EntityEvent::event_target`]
	/// targets this entity.
	///
	/// # Panics
	///
	/// If the entity has been despawned while this `EntityWorldMut` is still alive.
	///
	/// Panics if the given system is an exclusive system.
	// we need this because `observe` is restricted to [`EntityEvent`]
	fn observe_any<E: Event, B: Bundle, M>(
		&mut self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> &mut Self {
		// self.assert_not_despawned();
		let bundle = Observer::new(observer).with_entity(self.id());
		self.world_scope(move |world| {
			world.spawn(bundle);
		});
		self
	}
}



/// A [`Trigger`] for [`ActionEvent`] that behaves like [`PropagateEntityTrigger`], but
/// stores the `event_target` for ergonomics.
///
/// If `AUTO_PROPAGATE` is `true`, [`ActionTrigger::propagate`] will default to `true`.
pub struct ActionTrigger<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>>
{
	/// The [`Entity`] the [`Event`] is currently triggered for.
	pub event_target: Entity,
	/// The original [`Entity`] the [`Event`] was _first_ triggered for.
	pub original_event_target: Entity,

	/// Whether or not to continue propagating using the `T` [`Traversal`]. If this is false,
	/// The [`Traversal`] will stop on the current entity.
	pub propagate: bool,

	_marker: PhantomData<(E, T)>,
}

impl<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>>
	ActionTrigger<AUTO_PROPAGATE, E, T>
{
	/// Create a new [`ActionTrigger`] for the given target entity.
	pub fn new(event_target: Entity) -> Self {
		Self {
			event_target,
			original_event_target: event_target,
			propagate: AUTO_PROPAGATE,
			_marker: PhantomData,
		}
	}

	/// Get the current event target entity.
	pub fn event_target(&self) -> Entity { self.event_target }

	/// Get the original event target entity.
	pub fn original_event_target(&self) -> Entity { self.original_event_target }

	/// Whether or not to continue propagating using the `T` [`Traversal`]. If this is false,
	/// The [`Traversal`] will stop on the current entity.
	pub fn propagate(&self) -> bool { self.propagate }

	/// Set whether or not to continue propagating using the `T` [`Traversal`]. If this is false,
	/// The [`Traversal`] will stop on the current entity.
	pub fn set_propagate(&mut self, propagate: bool) -> &mut Self {
		self.propagate = propagate;
		self
	}
}


impl<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>> fmt::Debug
	for ActionTrigger<AUTO_PROPAGATE, E, T>
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActionTrigger")
			.field("event_target", &self.event_target)
			.field("original_event_target", &self.original_event_target)
			.field("propagate", &self.propagate)
			.field("_marker", &self._marker)
			.finish()
	}
}

unsafe impl<
	const AUTO_PROPAGATE: bool,
	E: for<'a> Event<Trigger<'a> = Self>,
	T: Traversal<E>,
> Trigger<E> for ActionTrigger<AUTO_PROPAGATE, E, T>
{
	unsafe fn trigger(
		&mut self,
		mut world: DeferredWorld,
		observers: &CachedObservers,
		trigger_context: &TriggerContext,
		event: &mut E,
	) {
		let mut current_entity = self.event_target;
		self.original_event_target = current_entity;
		// SAFETY:
		// - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
		// - the passed in event pointer comes from `event`, which is an `Event`
		// - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
		// - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
		unsafe {
			trigger_entity_internal(
				world.reborrow(),
				observers,
				event.into(),
				self.into(),
				current_entity,
				trigger_context,
			);
		}

		loop {
			if !self.propagate {
				return;
			}
			if let Ok(entity) = world.get_entity(current_entity)
				&& let Some(item) = entity.get_components::<T>()
				&& let Some(traverse_to) = T::traverse(item, event)
			{
				current_entity = traverse_to;
			} else {
				break;
			}

			self.event_target = current_entity;
			// SAFETY:
			// - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
			// - the passed in event pointer comes from `event`, which is an `Event`
			// - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
			// - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
			unsafe {
				trigger_entity_internal(
					world.reborrow(),
					observers,
					event.into(),
					self.into(),
					current_entity,
					trigger_context,
				);
			}
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[derive(ActionEvent)]
	struct MyEvent(String);

	#[test]
	fn works() {
		let store = Store::default();

		let mut world = World::new();
		world
			.spawn_empty()
			.observe_any(move |ev: On<MyEvent>| {
				store.set(ev.0.clone());
			})
			.trigger_action(MyEvent("bing bong".to_string()));
		store.get().xpect_eq("bing bong".to_string());
	}
}
