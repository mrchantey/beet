use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;

pub trait RunPayload: 'static + Send + Sync + Clone + Debug {
	type Result: ResultPayload<Run = Self>;
}
pub trait ResultPayload: 'static + Send + Sync + Clone + Debug {
	type Run: RunPayload<Result = Self>;
}

impl RunPayload for () {
	type Result = RunResult;
}
impl ResultPayload for RunResult {
	type Run = ();
}
impl<T: RunPayload> ActionEvent for OnRunAction<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}


/// An event triggered on *action observers*, never on the action entity (tree node) itself.
/// This should never be called directly, instead use [`OnRunLocal`] or [`OnRunGlobal`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnRun<T = ()> {
	pub payload: T,
	pub origin: Entity,
	pub action: Entity,
	// only OnRunAction is allowed to create this struct
	_sealed: (),
}

/// An event triggered on the action entities, propagated to the observers automatically.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnRunAction<T = ()> {
	pub payload: T,
	origin: Entity,
	action: Entity,
}

impl<T> OnRunAction<T> {
	/// Create a new OnRun trigger, called on the current entity.
	pub fn local(payload: T) -> Self {
		Self {
			payload,
			action: Entity::PLACEHOLDER,
			origin: Entity::PLACEHOLDER,
		}
	}
	pub fn local_with_origin(payload: T, origin: Entity) -> Self {
		Self {
			payload,
			origin,
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

impl<T: RunPayload> OnRun<T> {
	pub fn trigger_next(&self, mut commands: Commands, next_action: Entity) {
		commands.trigger(OnRunAction {
			payload: self.payload.clone(),
			origin: self.origin,
			action: next_action,
		});
	}
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

	pub fn trigger_result(&self, mut commands: Commands, payload: T::Result) {
		commands.trigger(OnResultAction::global_with_origin(
			self.action,
			self.origin,
			payload,
		));
	}
}

impl OnRun<()> {
	/// Usability helper, see [`OnRunLocal::new`].
	pub fn local() -> OnRunAction { OnRunAction::local(()) }
	/// Usability helper, see [`OnRunGlobal::new`].
	pub fn global(action: Entity) -> OnRunAction {
		OnRunAction::global(action, ())
	}
}

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
