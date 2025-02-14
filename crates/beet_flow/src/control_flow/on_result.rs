use crate::prelude::*;
use bevy::prelude::*;


/// An event triggered on an [`ActionEntity`], propagated to the observers automatically
/// with observers registered by the [run_plugin].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnResultAction<T = RunResult> {
	/// The payload of the result.
	/// By analogy if an action is a function, this would be the returned value.
	pub payload: T,
	/// this is not exposed as it may be placeholder, instead use [Trigger::resolve_origin]
	origin: Entity,
	/// this is not exposed as it may be placeholder, instead use [Trigger::resolve_action]
	action: Entity,
}

impl<T> ActionEvent for OnResultAction<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}

impl<T> OnResultAction<T> {
	/// Create a new [`OnResultAction`] event, where the origin
	/// may be a seperate entity from the action.
	/// ## Example
	/// ```
	/// # use beet_flow::doctest::*;
	/// # let mut world = world();
	/// let origin = world.spawn(Name::new("My Agent")).id();
	/// let action = world
	/// 	.spawn(Remove::<OnResult, Running>::default())
	/// 	.id();
	/// world.trigger(OnResultAction::new(action, origin, RunResult::Success));
	/// ```
	pub fn new(action: Entity, origin: Entity, payload: T) -> Self {
		Self {
			payload,
			origin,
			action,
		}
	}
	/// Convenience function to trigger directly on an [`ActionEntity`]
	/// where the origin is the [`ActionEntity`].
	/// ## Example
	/// ```
	/// # use beet_flow::doctest::*;
	/// # let mut world = world();
	/// world
	/// 	.spawn(Remove::<OnResult, Running>::default())
	/// 	.trigger(OnResultAction::local(RunResult::Success));
	/// ```
	pub fn local(payload: T) -> Self {
		Self {
			payload,
			origin: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		}
	}
	/// Convenience function to trigger globally for an existing [`ActionEntity`]
	/// where the origin is the [`ActionEntity`].
	/// ## Example
	/// ```
	/// # use beet_flow::doctest::*;
	/// # let mut world = world();
	/// let action = world
	/// 	.spawn(Remove::<OnResult, Running>::default())
	/// 	.id();
	/// world.trigger(OnResultAction::global(action, RunResult::Success));
	/// ```
	pub fn global(action: Entity, payload: T) -> Self {
		Self {
			payload,
			origin: action,
			action,
		}
	}
}

/// An event triggered on an [`ActionObserver`] which can be listened to
/// by actions.
///
/// It is not allowed to trigger this directly because that would
/// break the routing model of beet, instead see [OnResultAction].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnResult<T = RunResult> {
	/// The payload of the result.
	/// By analogy if an action is a function, this would be the returned value.
	pub payload: T,
	/// The entity upon which actions can perform some work, often the
	/// root of the action tree but can be any entity.
	pub origin: Entity,
	/// The [ActionEntity] that triggered this event.
	pub action: Entity,
	/// only [OnResultAction] is allowed to create this struct
	_sealed: (),
}

/// Called on the [Parent] of an [ActionEntity] to propagate the result,
/// See [HighestScore] for an example usage.
/// It is not allowed to trigger this directly because that would
/// break the routing model of beet, instead see [OnResultAction].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnChildResult<T = RunResult> {
	/// The payload of the result.
	/// By analogy if an action is a function, this would be the returned value.
	pub payload: T,
	/// The entity upon which actions can perform some work, often the
	/// root of the action tree but can be any entity.
	pub origin: Entity,
	/// The parent [ActionEntity] receiving the child result.
	pub parent: Entity,
	/// The child [ActionEntity] entity that triggered the result.
	pub child: Entity,
	/// only [OnResultAction] is allowed to create this struct
	_sealed: (),
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
					parent,
					child: action,
					_sealed: (),
				};
				commands.trigger_targets(res, (*action_observers).clone());
			}
		}
	}
	/// Create a new [`OnResultAction`] on [Self::parent] with the given payload.
	/// This is essentially a bubble up.
	pub fn trigger_bubble(&self, mut commands: Commands) {
		commands.trigger(OnResultAction::new(
			self.parent,
			self.origin,
			self.payload.clone(),
		));
	}
	/// Create a new [`OnRunAction`] on [Self::parent] with the given payload.
	/// This is essentially a bubble up.
	pub fn trigger_bubble_with(&self, mut commands: Commands, payload: T) {
		commands.trigger(OnResultAction::new(
			self.parent,
			self.origin,
			payload,
		));
	}

	/// Create a new [`OnRunAction`] on [Self::parent] with the given payload.
	pub fn trigger_run(
		&self,
		mut commands: Commands,
		next_action: Entity,
		next_payload: T::Run,
	) {
		commands.trigger(OnRunAction::new(
			next_action,
			self.origin,
			next_payload,
		));
	}
}


/// The default payload for [`OnResult`], this specifies the result of an action.
/// The Success/Failure pattern is commonly used by control flow actions in
/// the behavior tree pattern.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum RunResult {
	/// The action was successful.
	#[default]
	Success,
	/// The action failed.
	Failure,
}

/// Add this to an entity to prevent the run result from bubbling up.
/// Any action that requires this needs to manually call OnChildResult
/// on the parent entity. For an example, see [`Repeat`].
#[derive(Default, Component, Reflect)]
pub struct NoBubble;


/// Propagate the [`OnResultAction`] event to all [`ActionObservers`],
/// and call [`OnChildResult`] on the [`Parent`] if it exists.
pub(super) fn propagate_on_result<T: ResultPayload>(
	ev: Trigger<OnResultAction<T>>,
	mut commands: Commands,
	action_observers: Query<&ActionObservers>,
	should_bubble: Query<(), Without<NoBubble>>,
	parents: Query<&Parent>,
) {
	let action = ev.resolve_action();
	let origin = ev.resolve_origin();
	// propagate result to observers
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
