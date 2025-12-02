use crate::prelude::*;
use crate::types::ChildEnd;
use beet_core::prelude::*;
use bevy::ecs::event::Trigger;
use bevy::ecs::event::trigger_entity_internal;
use bevy::ecs::observer::CachedObservers;
use bevy::ecs::observer::TriggerContext;
use bevy::ecs::traversal::Traversal;
use std::fmt;
use std::marker::PhantomData;


pub trait ActionEvent:
	'static
	+ Send
	+ Sync
	+ for<'a> Event<Trigger<'a> = ActionTrigger<false, Self, &'static ChildOf>>
{
}
impl<T> ActionEvent for T where
	for<'a> T: 'static
		+ Send
		+ Sync
		+ Event<Trigger<'a> = ActionTrigger<false, T, &'static ChildOf>>
{
}


#[extend::ext(name=OnActionEventExt)]
pub impl<'w, 't, T> On<'w, 't, T>
where
	T: ActionEvent,
{
	/// The [`Entity`] this event is currently triggered for.
	fn action(&self) -> Entity { self.trigger().cx.action }
	/// The 'agent' entity for this action.
	/// Unless explicitly specified the agent is the first [`ActionOf`] in the
	/// action's ancestors (inclusive), or the root ancestor if no [`ActionOf`]
	/// is found.
	fn agent(&self) -> Entity { self.trigger().cx.agent }
	/// Trigger the event on this [`Action`] with this action's context.
	#[track_caller]
	fn trigger_with_cx(&mut self, event: impl ActionEvent) -> &mut Self {
		self.trigger_mut().trigger_with_cx(event);
		self
	}
	/// Trigger the event with the provided [`Action`] with this action's context.
	#[track_caller]
	fn trigger_action_with_cx(
		&mut self,
		action: Entity,
		event: impl ActionEvent,
	) -> &mut Self {
		self.trigger_mut().trigger_action_with_cx(action, event);
		self
	}

	fn run_async<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + Send + FnOnce(AsyncWorld, &mut ActionContext) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		self.trigger_mut().run_async(func);
		self
	}
}
#[extend::ext(name=OnChildEndActionEventExt)]
pub impl<'w, 't, T> On<'w, 't, ChildEnd<T>>
where
	T: Clone + ActionEvent,
{
	/// Trigger [`T`] on this [`event_target`], essentially propagating a
	/// [`ChildEnd<T>`] into a [`T`] event while tracking the [`ActionContext::agent`]
	fn propagate_child(&mut self) -> &mut Self {
		let ev = self
			.event()
			.clone()
			.inner()
			.with_agent(self.trigger().agent());
		self.trigger_mut().trigger_target(ev);
		self
	}
}




/// A [`Trigger`] for [`ActionEvent`] that behaves like [`PropagateEntityTrigger`], but
/// stores the `event_target` for ergonomics.
///
/// If `AUTO_PROPAGATE` is `true`, [`ActionTrigger::propagate`] will default to `true`.
pub struct ActionTrigger<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>>
{
	/// The context for the current [`Action`]
	cx: ActionContext,
	/// Whether or not to continue propagating using the `T` [`Traversal`]. If this is false,
	/// The [`Traversal`] will stop on the current entity.
	pub propagate: bool,
	_marker: PhantomData<(E, T)>,
}

impl<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>> std::ops::Deref
	for ActionTrigger<AUTO_PROPAGATE, E, T>
{
	type Target = ActionContext;

	fn deref(&self) -> &Self::Target { &self.cx }
}

impl<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>> std::ops::DerefMut
	for ActionTrigger<AUTO_PROPAGATE, E, T>
{
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.cx }
}


impl<const AUTO_PROPAGATE: bool, E: Event, T: Traversal<E>>
	ActionTrigger<AUTO_PROPAGATE, E, T>
{
	/// Create a new [`ActionTrigger`] for the given target entity.
	pub fn new(cx: ActionContext) -> Self {
		Self {
			cx,
			propagate: AUTO_PROPAGATE,
			_marker: PhantomData,
		}
	}

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
			.field("cx", &self.cx)
			.field("propagate", &self.propagate)
			.field("_marker", &self._marker)
			.finish()
	}
}

impl<
	const AUTO_PROPAGATE: bool,
	E: for<'a> Event<Trigger<'a> = Self>,
	T: Traversal<E>,
> TriggerFromTarget for ActionTrigger<AUTO_PROPAGATE, E, T>
{
	fn trigger_from_target(entity: Entity) -> Self {
		// agent will be found in the trigger
		Self::new(ActionContext::new_no_agent(entity))
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
		if self.cx.agent == Entity::PLACEHOLDER {
			self.cx.agent = self.cx.find_agent(&world);
		}

		let mut current_entity = self.cx.action;
		// self.original_event_target = current_entity;

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
		world.commands().append(&mut self.cx.queue);

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

			self.cx.action = current_entity;
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
	use beet_core::prelude::*;
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
			.trigger_target(MyEvent("bing bong".to_string()));
		store.get().xpect_eq("bing bong".to_string());
	}
	#[test]
	fn tracks_caller() {
		let store = Store::default();

		let mut world = World::new();
		world
			.spawn_empty()
			.observe_any(move |ev: On<MyEvent>| {
				store.set(ev.0.clone());
			})
			.trigger_target(MyEvent("bing bong".to_string()));
		store.get().xpect_eq("bing bong".to_string());
	}
}
