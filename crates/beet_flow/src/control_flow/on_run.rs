use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;


/// An event triggered on an [`ActionEntity`], propagated to the observers automatically
/// with observers registered by the [run_plugin].
///
/// This can be triggered on any entity, and [`OnRun`] will be propagated to [`Self::action`].
/// - If [`Self::action`] is [`Entity::PLACEHOLDER`], the entity this was triggered on will be used.
/// - If the action is local and the trigger is global, ie `commands.trigger(OnRunAction::local(()))`
/// 	this will result in a panic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnRunAction<T = ()> {
	/// The payload of the run.
	/// By analogy if an action is a function, this would be the arguments.
	pub payload: T,
	/// this is not exposed as it may be placeholder
	/// - to set use [OnRunAction::new]
	/// - to get use [Trigger::resolve_action]
	origin: Entity,
	/// this is not exposed as it may be placeholder
	/// - to set use [OnRunAction::new]
	/// - to get use [Trigger::resolve_action]
	action: Entity,
}

impl<T> ActionEvent for OnRunAction<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}

/// Create a local [`OnRunAction`] event with a default payload.
impl<T: Default> Default for OnRunAction<T> {
	fn default() -> Self { Self::local(Default::default()) }
}

impl<T> OnRunAction<T> {
	/// Create a new [`OnRunAction`] event, where the origin
	/// may be a seperate entity from the action.
	/// ## Example
	/// ```
	/// # use beet_flow::doctest::*;
	/// # let mut world = world();
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
	/// ## Example
	/// ```
	/// # use beet_flow::doctest::*;
	/// # let mut world = world();
	/// world
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
	/// ## Example
	/// ```
	/// # use beet_flow::doctest::*;
	/// # let mut world = world();
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
	pub fn trigger_next(&self, commands: &mut Commands, next_action: Entity) {
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
	pub fn trigger_result(&self, commands: &mut Commands, payload: T::Result) {
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
	/// ## Example
	/// ```
	/// # use beet_flow::doctest::*;
	/// # let mut world = world();
	/// world
	/// 	.spawn(ReturnWith(RunResult::Success))
	/// 	.trigger(OnRun::local());
	/// ```
	pub fn local() -> OnRunAction { OnRunAction::local(()) }
	/// Convenience function to trigger globally for an existing [`ActionEntity`]
	/// where the origin is the [`ActionEntity`].
	/// ## Example
	/// ```
	/// # use beet_flow::doctest::*;
	/// # let mut world = world();
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
///
/// The nature of this routing techique allows [`OnRunAction`] to be called
/// on any entity, ie a different entity to the [`OnRunAction::action`].
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


/// Some actions provide the option to specify a target to perform
/// an operation on, for example [`Insert`] and [`Remove`].
#[derive(Debug, Default, Clone, Reflect)]
pub enum TargetEntity {
	/// Use The `action` entity as the target
	#[default]
	Action,
	/// Use the `origin` entity as the target
	Origin,
	/// Use some other entity as the target
	Other(Entity),
}

impl TargetEntity {
	/// Get the target entity for this event.
	pub fn get_target(&self, ev: &impl ObserverEvent) -> Entity {
		match self {
			TargetEntity::Action => ev.action(),
			TargetEntity::Origin => ev.origin(),
			TargetEntity::Other(entity) => *entity,
		}
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
