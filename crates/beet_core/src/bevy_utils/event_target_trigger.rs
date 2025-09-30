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


/// An [`EntityEvent`] [`Trigger`] that behaves like [`PropagateEntityTrigger`], but
/// stores the `event_target` for ergonomics.
///
/// If `AUTO_PROPAGATE` is `true`, [`EventTargetTrigger::propagate`] will default to `true`.
pub struct EventTargetTrigger<
	const AUTO_PROPAGATE: bool,
	E: Event,
	T: Traversal<E>,
> {
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
	EventTargetTrigger<AUTO_PROPAGATE, E, T>
{
	/// Create a new [`EventTargetTrigger`] for the given target entity.
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
	for EventTargetTrigger<AUTO_PROPAGATE, E, T>
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("EventTargetTrigger")
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
> Trigger<E> for EventTargetTrigger<AUTO_PROPAGATE, E, T>
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


#[extend::ext(name=CommandsEventTargetTriggerExt)]
pub impl EntityCommands<'_> {
	fn auto_trigger<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = EventTargetTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Send + Sync + Traversal<E>,
	>(
		&mut self,
		mut ev: E,
	) -> &mut Self {
		let caller = MaybeLocation::caller();
		let mut trigger = EventTargetTrigger::new(self.id());
		self.queue(move |mut world_scope: EntityWorldMut| {
			world_scope.auto_trigger_ref(&mut ev, &mut trigger, caller);
		});
		self
	}
	/// An [`EntityCommand`] that creates an [`Observer`](crate::observer::Observer)
	/// watching for an [`EntityEvent`] of type `E` whose [`EntityEvent::event_target`]
	/// targets this entity.
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


#[extend::ext(name=EntityWorldMutEventTargetTriggerExt)]
pub impl EntityWorldMut<'_> {
	fn auto_trigger<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = EventTargetTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Traversal<E>,
	>(
		&mut self,
		mut ev: E,
	) -> &mut Self {
		let caller = MaybeLocation::caller();
		let mut trigger =
			EventTargetTrigger::<AUTO_PROPAGATE, E, T>::new(self.id());
		self.auto_trigger_ref(&mut ev, &mut trigger, caller);
		self
	}
	fn auto_trigger_ref<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = EventTargetTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Traversal<E>,
	>(
		&mut self,
		ev: &mut E,
		trigger: &mut E::Trigger<'a>,
		caller: MaybeLocation,
	) -> &mut Self {
		self.world_scope(move |world| {
			let event_key = world.register_event_key::<E>();
			// SAFETY: event_key was just registered and matches `event`
			unsafe {
				DeferredWorld::from(world)
					.trigger_raw(event_key, ev, trigger, caller);
			}
		});
		self.world_scope(|world| world.flush());
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


pub fn prevent_auto_propagate<
	const AUTO_PROPAGATE: bool,
	E: for<'a> Event<Trigger<'a> = EventTargetTrigger<AUTO_PROPAGATE, E, T>>,
	T: 'static + Traversal<E>,
>(
	mut world: DeferredWorld,
	cx: HookContext,
) {
	world
		.commands()
		.entity(cx.entity)
		.observe_any(move |mut ev: On<E>| {
			ev.trigger_mut().set_propagate(false);
		});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[derive(EntityTargetEvent)]
	struct MyEvent;

	#[test]
	fn works() {
		let mut world = World::new();
		let called = Store::default();
		world
			.spawn((
				Name::new("foo"),
				EntityObserver::new(
					move |ev: On<MyEvent>, names: Query<&Name>| {
						names
							.get(ev.trigger().event_target())
							.unwrap()
							.to_string()
							.xpect_eq("foo");
						called.set(true);
					},
				),
			))
			.auto_trigger(MyEvent);
		called.get().xpect_eq(true);
	}
}
