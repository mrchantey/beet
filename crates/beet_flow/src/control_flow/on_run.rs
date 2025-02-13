use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;


/// An event triggered on an [`ActionEntity`], propagated to the observers automatically
/// with observers registered by the [run_plugin].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnRunAction<T = ()> {
	/// The payload of the run.
	/// By analogy if an action is a function, this would be the arguments.
	pub payload: T,
	/// this is not exposed as it may be placeholder, instead use [Trigger::resolve_origin]
	origin: Entity,
	/// this is not exposed as it may be placeholder, instead use [Trigger::resolve_action]
	action: Entity,
}

impl<T> ActionEvent for OnRunAction<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}

impl<T> OnRunAction<T> {
	/// Create a new [`OnRunAction`] event, where the origin
	/// may be a seperate entity from the action.
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_flow::prelude::*;
	/// let mut world = World::new();
	/// let origin = world.spawn(Name::new("My Agent")).id();
	/// let action = world
	/// 	.spawn(ReturnWith(RunResult::Success))
	/// 	.id();
	/// world.trigger(OnRunAction::new(action, origin, ()));
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
	/// When triggering the default [`OnRun<()>`], prefer using [`OnRun::local`].
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_flow::prelude::*;
	/// World::new()
	/// 	.spawn(ReturnWith(RunResult::Success))
	/// 	.trigger(OnRunAction::local(()));
	/// ```
	pub fn local(payload: T) -> Self {
		Self {
			payload,
			action: Entity::PLACEHOLDER,
			origin: Entity::PLACEHOLDER,
		}
	}
	/// Convenience function to trigger globally for an existing [`ActionEntity`]
	/// where the origin is the [`ActionEntity`].
	/// When triggering the default [`OnRun<()>`], prefer using [`OnRun::global`].
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_flow::prelude::*;
	/// let mut world = World::new();
	/// let action = world
	/// 	.spawn(ReturnWith(RunResult::Success))
	/// 	.id();
	/// world.trigger(OnRunAction::global(action, ()));
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
/// break the routing model of beet, instead see [OnRunAction].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnRun<T = ()> {
	/// The payload of the run.
	/// By analogy if an action is a function, this would be the arguments.
	pub payload: T,
	/// The entity upon which actions can perform some work, often the
	/// root of the action tree but can be any entity.
	pub origin: Entity,
	/// The [ActionEntity] that triggered this event.
	pub action: Entity,
	// only OnRunAction is allowed to create this struct
	_sealed: (),
}

impl<T: RunPayload> OnRun<T> {
	/// Call [`OnRunAction`] for the provided action, cloning this event's
	/// origin and payload.
	pub fn trigger_next(&self, mut commands: Commands, next_action: Entity) {
		commands.trigger(OnRunAction {
			payload: self.payload.clone(),
			origin: self.origin,
			action: next_action,
		});
	}
	/// Call [`OnRunAction`] for the provided action and payload, cloning this event's
	/// origin.
	pub fn trigger_next_with(
		&self,
		mut commands: Commands,
		next_action: Entity,
		next_payload: T,
	) {
		commands.trigger(OnRunAction {
			payload: next_payload,
			origin: self.origin,
			action: next_action,
		});
	}
	/// Call [`OnResultAction`] for this event's action, cloning this event's
	/// origin.
	pub fn trigger_result(&self, mut commands: Commands, payload: T::Result) {
		commands.trigger(OnResultAction::new(
			self.action,
			self.origin,
			payload,
		));
	}
}

impl OnRun<()> {
	/// Convenience function to trigger directly on an [`ActionEntity`]
	/// where the origin is the [`ActionEntity`].
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_flow::prelude::*;
	/// World::new()
	/// 	.spawn(ReturnWith(RunResult::Success))
	/// 	.trigger(OnRun::local());
	/// ```
	pub fn local() -> OnRunAction { OnRunAction::local(()) }
	/// Convenience function to trigger globally for an existing [`ActionEntity`]
	/// where the origin is the [`ActionEntity`].
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_flow::prelude::*;
	/// let mut world = World::new();
	/// let action = world
	/// 	.spawn(ReturnWith(RunResult::Success))
	/// 	.id();
	/// world.trigger(OnRun::global(action));
	/// ```
	pub fn global(action: Entity) -> OnRunAction {
		OnRunAction::global(action, ())
	}
}

/// Propagate the [`OnRunAction`] event to all [`ActionObservers`].
pub(crate) fn propagate_on_run<T: RunPayload>(
	ev: Trigger<OnRunAction<T>>,
	mut commands: Commands,
	action_observers: Query<&ActionObservers>,
) {
	let action = ev.resolve_action();
	if let Ok(observers) = action_observers.get(action) {
		// OnRunLocal::new uses placeholder, replace with action entity
		let origin = ev.resolve_origin();
		commands.trigger_targets(
			OnRun {
				payload: ev.payload.clone(),
				origin,
				action,
				_sealed: (),
			},
			(**observers).clone(),
		);
	}
}





#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[action(trigger_count)]
	#[derive(Default, Component)]
	struct TriggerCount(i32);

	fn trigger_count(
		trigger: Trigger<OnRun>,
		mut query: Query<&mut TriggerCount>,
	) {
		query.get_mut(trigger.action).unwrap().as_mut().0 += 1;
	}

	#[test]
	fn local() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());

		let entity = app
			.world_mut()
			.spawn(TriggerCount::default())
			.flush_trigger(OnRun::local())
			.id();

		expect(app.world().get::<TriggerCount>(entity).unwrap().0).to_be(1);
	}
	#[test]
	fn global() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());

		let action = app.world_mut().spawn(TriggerCount::default()).id();
		app.world_mut().flush_trigger(OnRun::global(action));

		expect(app.world().get::<TriggerCount>(action).unwrap().0).to_be(1);
	}
}
