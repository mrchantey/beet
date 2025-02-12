use crate::prelude::*;
use bevy::prelude::*;


/// An event triggered on the action entities, propagated to the observers automatically.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnResultAction<T = RunResult> {
	pub payload: T,
	/// this is not exposed as it may be placeholder, instead use [Trigger::resolve_origin]
	origin: Entity,
	/// this is not exposed as it may be placeholder, instead use [Trigger::resolve_action]
	action: Entity,
}

impl<T: ResultPayload> ActionEvent for OnResultAction<T> {
	fn _action(&self) -> Entity { self.action }
	fn _origin(&self) -> Entity { self.origin }
}

impl<T: ResultPayload> OnResultAction<T> {
	pub fn local(payload: T) -> Self {
		Self {
			payload,
			origin: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		}
	}
	pub fn global(action: Entity, payload: T) -> Self {
		Self {
			payload,
			origin: action,
			action,
		}
	}
	pub fn global_with_origin(
		action: Entity,
		origin: Entity,
		payload: T,
	) -> Self {
		Self {
			payload,
			origin,
			action,
		}
	}
}


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnResult<T = RunResult> {
	pub payload: T,
	pub origin: Entity,
	pub action: Entity,
	// only OnResultAction is allowed to create this struct
	_sealed: (),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnChildResult<T = RunResult> {
	pub payload: T,
	pub origin: Entity,
	pub action: Entity,
	pub child: Entity,
}

impl<T: ResultPayload> OnChildResult<T> {
	/// Call [OnChildResult] on the action's parent entity.
	/// This is called by default unless [NoBubble] is present
	/// on the action entity.
	pub fn try_trigger(
		mut commands: Commands,
		parents: Query<&Parent>,
		action_observers: Query<&ActionObservers>,
		action: Entity,
		origin: Entity,
		payload: T,
	) {
		if let Ok(parent) = parents.get(action) {
			let parent = parent.get();
			if let Ok(action_observers) = action_observers.get(parent) {
				let res = OnChildResult {
					payload,
					origin,
					action: parent,
					child: action,
				};
				commands.trigger_targets(res, (*action_observers).clone());
			}
		}
	}

	pub fn trigger_bubble(&self, mut commands: Commands) {
		commands.trigger(OnResultAction::global_with_origin(
			self.action,
			self.origin,
			self.payload.clone(),
		));
	}
	pub fn trigger_bubble_with(&self, mut commands: Commands, payload: T) {
		commands.trigger(OnResultAction::global_with_origin(
			self.action,
			self.origin,
			payload,
		));
	}
	pub fn trigger_run(
		&self,
		mut commands: Commands,
		next_action: Entity,
		next_payload: T::Run,
	) {
		commands.trigger(OnRunAction::global_with_origin(
			next_action,
			self.origin,
			next_payload,
		));
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum RunResult {
	#[default]
	Success,
	Failure,
}

/// Add this to an entity to prevent the run result from bubbling up.
/// Any action that requires this needs to manually call OnChildResult
/// on the parent entity. For an example, see [`RepeatFlow`].
#[derive(Default, Component, Reflect)]
// do we need this?
pub struct NoBubble;

impl<T: ResultPayload> OnResult<T> {}



/// Global observer to pass an action up to all *parent observers*,
/// so they may handle the response.
///
/// Unlike [propagate_request_to_observers], this is called on parent
/// observers.
///
///
pub fn propagate_on_result<T: ResultPayload>(
	ev: Trigger<OnResultAction<T>>,
	mut commands: Commands,
	action_observers: Query<&ActionObservers>,
	should_bubble: Query<(), Without<NoBubble>>,
	parents: Query<&Parent>,
) {
	let action = ev.resolve_action();
	let origin = ev.resolve_origin();
	if let Ok(action_observers) = action_observers.get(action) {
		let res = OnResult {
			payload: ev.payload.clone(),
			origin,
			action,
			_sealed: (),
		};
		commands.trigger_targets(res, (*action_observers).clone());
	}

	// propagate result to parents
	if should_bubble.contains(action) {
		OnChildResult::try_trigger(
			commands,
			parents,
			action_observers,
			action,
			ev.resolve_origin(),
			ev.payload.clone(),
		);
	}
}
