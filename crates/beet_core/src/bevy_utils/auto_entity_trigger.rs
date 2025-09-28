use crate::prelude::*;
use bevy::ecs::event::Trigger;
use bevy::ecs::event::trigger_entity_internal;
use bevy::ecs::observer::CachedObservers;
use bevy::ecs::observer::TriggerContext;
use bevy::ecs::traversal::Traversal;
use bevy::ecs::world::DeferredWorld;
use std::fmt;
use std::marker::PhantomData;


/// An [`EntityEvent`] [`Trigger`] that behaves like [`PropagateEntityTrigger`], but
/// stores the `event_target` for ergonomics.
///
/// If `AUTO_PROPAGATE` is `true`, [`AutoEntityTrigger::propagate`] will default to `true`.
pub struct AutoEntityTrigger<
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
	AutoEntityTrigger<AUTO_PROPAGATE, E, T>
{
	/// Create a new [`AutoEntityTrigger`] for the given target entity.
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
	for AutoEntityTrigger<AUTO_PROPAGATE, E, T>
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("AutoEntityTrigger")
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
> Trigger<E> for AutoEntityTrigger<AUTO_PROPAGATE, E, T>
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
