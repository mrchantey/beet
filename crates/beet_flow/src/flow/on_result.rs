use crate::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnResult<T = RunResult> {
	pub payload: T,
	pub origin: Entity,
	pub action: Entity,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnChildResult<T = RunResult> {
	pub payload: T,
	pub origin: Entity,
	pub action: Entity,
	pub child: Entity,
}

impl<T: ResultPayload> OnChildResult<T> {
	pub fn trigger_bubble(&self, mut commands: Commands) {
		commands.entity(self.action).trigger(OnResult {
			payload: self.payload.clone(),
			origin: self.origin,
			action: self.action,
		});
	}
	pub fn trigger_bubble_with(&self, mut commands: Commands, payload: T) {
		commands.entity(self.action).trigger(OnResult {
			payload,
			origin: self.origin,
			action: self.action,
		});
	}
	pub fn trigger_run(
		&self,
		mut commands: Commands,
		next_action: Entity,
		next_payload: T::Run,
	) {
		commands
			.entity(next_action)
			.trigger(OnRunAction::local_with_origin(next_payload, self.origin));
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum RunResult {
	#[default]
	Success,
	Failure,
}

/// Add this to an entity to prevent the run result from bubbling up.
///
/// Any action that requires this needs to manually call OnChildResult
/// on the parent entity. For an example, see [`RepeatFlow`].
#[derive(Default, Component, Reflect)]
pub struct NoBubble;

impl<T: ResultPayload> OnResult<T> {
	pub fn new_local(payload: T) -> Self {
		Self {
			payload,
			origin: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		}
	}
	pub fn new_global(action: Entity, payload: T) -> Self {
		Self {
			payload,
			origin: action,
			action,
		}
	}
}



/// Global observer to pass an action up to all *parent observers*,
/// so they may handle the response.
///
/// Unlike [propagate_request_to_observers], this is called on parent
/// observers.
pub fn propagate_on_result<T: ResultPayload>(
	res: Trigger<OnResult<T>>,
	mut commands: Commands,
	action_observers: Query<&ActionObservers>,
	action_observer_markers: Query<(), With<ActionObserverMarker>>,
	no_bubble: Query<(), With<NoBubble>>,
	parents: Query<&Parent>,
) {
	if action_observer_markers.contains(res.entity())
		|| no_bubble.contains(res.action)
	{
		return;
	}

	if let Ok(parent) = parents.get(res.action) {
		let parent = parent.get();
		if let Ok(action_observers) = action_observers.get(parent) {
			let res = OnChildResult {
				payload: res.payload.clone(),
				origin: res.origin,
				action: parent,
				child: res.action,
			};
			commands.trigger_targets(res, (*action_observers).clone());
		}
	}
}
