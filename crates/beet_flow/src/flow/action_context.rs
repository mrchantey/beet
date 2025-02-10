use crate::prelude::*;
use bevy::prelude::*;


/// marker trait
pub trait ActionPayload: 'static + Send + Sync + Clone {}


pub type ActionTrigger<'w, T> = Trigger<'w, ActionContext<T>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct ActionContext<T> {
	pub payload: T,
	/// Aka agent,target.
	pub origin: Entity,
	pub action: Entity,
}

impl<T: Default> Default for ActionContext<T> {
	fn default() -> Self {
		Self {
			payload: Default::default(),
			origin: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		}
	}
}

impl<T> ActionContext<T> {
	pub fn placeholder(payload: T) -> Self {
		Self {
			payload,
			origin: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		}
	}
	pub fn new(payload: T) -> Self {
		Self {
			payload,
			origin: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		}
	}
	pub fn new_with_action(action: Entity, payload: T) -> Self {
		Self {
			payload,
			origin: action,
			action,
		}
	}
	pub fn new_with_action_and_target(
		action: Entity,
		target: Entity,
		payload: T,
	) -> Self {
		Self {
			payload,
			origin: target,
			action,
		}
	}
}

impl<T: Request> ActionContext<T> {
	pub fn into_response(&self, payload: T::Res) -> ActionContext<T::Res> {
		ActionContext {
			payload,
			origin: self.origin,
			action: self.action,
		}
	}
}

#[extend::ext(name=MyTypeExt)]
pub impl<T: ActionPayload + Default> T {
	fn default_trigger() -> ActionContext<Self>
	where
		Self: Sized,
	{
		ActionContext::placeholder(Self::default())
	}
	fn trigger(self) -> ActionContext<Self>
	where
		Self: Sized,
	{
		ActionContext::placeholder(self)
	}
	fn trigger_for_action(self, action: Entity) -> ActionContext<Self>
	where
		Self: Sized,
	{
		ActionContext::new_with_action(action, self)
	}
	fn trigger_for_action_and_origin(
		self,
		action: Entity,
		origin: Entity,
	) -> ActionContext<Self>
	where
		Self: Sized,
	{
		ActionContext::new_with_action_and_target(action, origin, self)
	}
}
