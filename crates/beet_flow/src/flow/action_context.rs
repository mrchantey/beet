use bevy::prelude::*;
use std::fmt::Debug;

/// marker trait
pub trait ActionPayload: 'static + Send + Sync + Clone + Debug {}

/// Wrapper for all action triggers, providing context alongiside
/// the payload, including the action (behavior tree node) that the trigger was called
/// for, and the origin (agent).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct On<T> {
	pub payload: T,
	/// Aka agent,target.
	pub origin: Entity,
	pub action: Entity,
	/// The action previously triggered.
	/// For a request this is the parent,
	/// for a response this is the child.
	pub prev_action: Entity,
}

impl<T: ActionPayload> On<T> {
	pub fn trigger_next_with<U: ActionPayload>(
		&self,
		mut commands: Commands,
		next_action: Entity,
		next_payload: U,
	) {
		commands.entity(next_action).trigger(On {
			payload: next_payload,
			action: next_action,
			origin: self.origin,
			prev_action: self.action,
		});
	}
}

impl<T: Default> Default for On<T> {
	fn default() -> Self {
		Self {
			payload: Default::default(),
			origin: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
			prev_action: Entity::PLACEHOLDER,
		}
	}
}

impl<T> On<T> {
	pub fn placeholder(payload: T) -> Self {
		Self {
			payload,
			origin: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
			prev_action: Entity::PLACEHOLDER,
		}
	}
	pub fn new(payload: T) -> Self {
		Self {
			payload,
			origin: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
			prev_action: Entity::PLACEHOLDER,
		}
	}
	pub fn new_with_action(action: Entity, payload: T) -> Self {
		Self {
			payload,
			origin: action,
			action,
			prev_action: Entity::PLACEHOLDER,
		}
	}
	pub fn new_with_action_and_target(
		action: Entity,
		origin: Entity,
		payload: T,
	) -> Self {
		Self {
			payload,
			origin,
			action,
			prev_action: Entity::PLACEHOLDER,
		}
	}
}

#[extend::ext(name=MyTypeExt)]
pub impl<T: ActionPayload + Default> T {
	fn default_trigger() -> On<Self>
	where
		Self: Sized,
	{
		On::placeholder(Self::default())
	}
	fn trigger(self) -> On<Self>
	where
		Self: Sized,
	{
		On::placeholder(self)
	}
	fn trigger_for_action(self, action: Entity) -> On<Self>
	where
		Self: Sized,
	{
		On::new_with_action(action, self)
	}
	fn trigger_for_action_and_origin(
		self,
		action: Entity,
		origin: Entity,
	) -> On<Self>
	where
		Self: Sized,
	{
		On::new_with_action_and_target(action, origin, self)
	}
}
