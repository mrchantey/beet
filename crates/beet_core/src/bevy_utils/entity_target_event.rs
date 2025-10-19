use std::fmt;

use crate::prelude::*;
use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::event::Trigger;
use bevy::ecs::event::trigger_entity_internal;
use bevy::ecs::observer::CachedObservers;
use bevy::ecs::observer::TriggerContext;
use bevy::ecs::system::IntoObserverSystem;

/// Copied verbatim from [`PropagateEntityTrigger`] but storing the `event_target` on the [`Trigger`] not the [`Event`].
/// An [`EntityEvent`] [`Trigger`] that behaves like [`EntityTrigger`], but "propagates" the event
/// using an [`Entity`] [`Traversal`]. At each step in the propagation, the [`EntityTrigger`] logic will
/// be run, until [`EntityTargetTrigger::propagate`] is false, or there are no entities left to traverse.
///
/// This is used by the [`EntityEvent`] derive when `#[entity_event(propagate)]` is enabled. It is usable by every
/// [`EntityEvent`] type.
///
/// If `AUTO_PROPAGATE` is `true`, [`EntityTargetTrigger::propagate`] will default to `true`.
pub struct EntityTargetTrigger<
	const AUTO_PROPAGATE: bool,
	E: Event,
	T: Traversal<E>,
> {
	/// The original [`Entity`] the [`Event`] was _first_ triggered for.
	pub original_event_target: Entity,
	/// [`Entity`] the [`Event`] is _currently_ triggered for.
	pub event_target: Entity,

	/// Whether or not to continue propagating using the `T` [`Traversal`]. If this is false,
	/// The [`Traversal`] will stop on the current entity.
	pub propagate: bool,

	_marker: PhantomData<(E, T)>,
}

impl<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>>
	EntityTargetTrigger<AUTO_PROPAGATE, E, T>
{
	fn new(event_target: Entity) -> Self {
		Self {
			original_event_target: event_target,
			event_target,
			propagate: AUTO_PROPAGATE,
			_marker: Default::default(),
		}
	}
}

impl<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>> fmt::Debug
	for EntityTargetTrigger<AUTO_PROPAGATE, E, T>
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("EntityTargetTrigger")
			.field("original_event_target", &self.original_event_target)
			.field("propagate", &self.propagate)
			.field("_marker", &self._marker)
			.finish()
	}
}

// SAFETY:
// - `E`'s [`Event::Trigger`] is constrained to [`EntityTargetTrigger<E>`]
unsafe impl<
	const AUTO_PROPAGATE: bool,
	E: for<'a> Event<Trigger<'a> = Self>,
	T: Traversal<E>,
> Trigger<E> for EntityTargetTrigger<AUTO_PROPAGATE, E, T>
{
	unsafe fn trigger(
		&mut self,
		mut world: DeferredWorld,
		observers: &CachedObservers,
		trigger_context: &TriggerContext,
		event: &mut E,
	) {
		// SAFETY:
		// - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
		// - the passed in event pointer comes from `event`, which is an `Event`
		// - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
		// - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
		unsafe {
			let target = self.event_target;
			trigger_entity_internal(
				world.reborrow(),
				observers,
				event.into(),
				self.into(),
				target,
				trigger_context,
			);
		}

		loop {
			if !self.propagate {
				return;
			}
			if let Ok(entity) = world.get_entity(self.event_target)
				&& let Some(item) = entity.get_components::<T>()
				&& let Some(traverse_to) = T::traverse(item, event)
			{
				self.event_target = traverse_to;
			} else {
				break;
			}
			// SAFETY:
			// - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
			// - the passed in event pointer comes from `event`, which is an `Event`
			// - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
			// - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
			unsafe {
				let target = self.event_target;
				trigger_entity_internal(
					world.reborrow(),
					observers,
					event.into(),
					self.into(),
					target,
					trigger_context,
				);
			}
		}
	}
}

/// An encompasing trait that includes all entity-like events:
/// - [`EntityEvent`]
/// - [`EntityTargetEvent`]
/// - [`ActionEvent`]
pub trait IntoEntityTargetEvent<M>: 'static + Send + Sync {
	type Event: for<'a> Event<Trigger<'a> = Self::Trigger>;
	type Trigger: 'static + Send + Sync + Trigger<Self::Event>;

	fn into_entity_target_event(
		self,
		entity: Entity,
	) -> (Self::Event, Self::Trigger);
}

/// parity with the vanilla `commands.entity().trigger(..)`
pub struct FnOnceIntoEntityTargetMarker;

impl<F, E, T> IntoEntityTargetEvent<(E, T, FnOnceIntoEntityTargetMarker)> for F
where
	F: 'static + Send + Sync + FnOnce(Entity) -> E,
	E: 'static + Send + Sync + for<'a> Event<Trigger<'a> = T>,
	T: 'static + Send + Sync + Default + Trigger<E>,
{
	type Event = E;
	type Trigger = T;

	fn into_entity_target_event(
		self,
		entity: Entity,
	) -> (Self::Event, Self::Trigger) {
		(self(entity), default())
	}
}

pub struct TriggerFromTargetIntoEntityTargetMarker;


impl<E, T> IntoEntityTargetEvent<(T, TriggerFromTargetIntoEntityTargetMarker)>
	for E
where
	E: 'static + Send + Sync + for<'a> Event<Trigger<'a> = T>,
	T: 'static + Send + Sync + Trigger<E> + TriggerFromTarget,
{
	type Event = E;
	type Trigger = T;

	fn into_entity_target_event(
		self,
		entity: Entity,
	) -> (Self::Event, Self::Trigger) {
		(self, T::trigger_from_target(entity))
	}
}



pub trait TriggerFromTarget {
	fn trigger_from_target(entity: Entity) -> Self;
}

impl<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>> TriggerFromTarget
	for EntityTargetTrigger<AUTO_PROPAGATE, E, T>
{
	fn trigger_from_target(entity: Entity) -> Self { Self::new(entity) }
}

#[extend::ext(name=EntityWorldMutActionEventExt)]
pub impl EntityWorldMut<'_> {
	#[track_caller]
	fn trigger_target<M>(
		&mut self,
		ev: impl IntoEntityTargetEvent<M>,
	) -> &mut Self {
		let (mut ev, mut trigger) = ev.into_entity_target_event(self.id());
		let caller = MaybeLocation::caller();
		self.world_scope(move |world| {
			world.trigger_ref_with_caller_pub(&mut ev, &mut trigger, caller);
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




#[extend::ext(name=EntityCommandsActionEventExt)]
pub impl EntityCommands<'_> {
	#[track_caller]
	fn trigger_target<M>(
		&mut self,
		ev: impl IntoEntityTargetEvent<M>,
	) -> &mut Self {
		let caller = MaybeLocation::caller();
		self.queue(move |mut entity: EntityWorldMut| {
			let (mut ev, mut trigger) =
				ev.into_entity_target_event(entity.id());
			entity.world_scope(move |world| {
				world.trigger_ref_with_caller_pub(
					&mut ev,
					&mut trigger,
					caller,
				);
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

#[extend::ext(name=OnEntityTargetEventExt)]
pub impl<'w, 't, const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>>
	On<'w, 't, E>
where
	E: for<'a> Event<Trigger<'a> = EntityTargetTrigger<AUTO_PROPAGATE, E, T>>,
{
	fn target(&self) -> Entity { self.trigger().event_target }
	fn original_target(&self) -> Entity {
		self.trigger().original_event_target
	}
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
		let entity = world.spawn_empty().id();
		let store = Store::new(Entity::PLACEHOLDER);
		world.add_observer(move |ev: On<MyEvent>| {
			store.set(ev.target());
		});
		world.entity_mut(entity).trigger_target(MyEvent).flush();
		store.get().xpect_eq(entity);
	}
}
