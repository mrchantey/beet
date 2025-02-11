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

/// This should never be called directly, instead use [`OnRunLocal`] or [`OnRunGlobal`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnRun<T = ()> {
	pub payload: T,
	pub origin: Entity,
	pub action: Entity,
	// ensure users can't create this directly
	_sealed: (),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnRunLocal<T = ()> {
	pub payload: T,
	pub origin: Entity,
}

impl<T> OnRunLocal<T> {
	/// Create a new OnRun trigger, called on the current entity.
	pub fn new(payload: T) -> OnRunLocal<T> {
		OnRunLocal {
			payload,
			origin: Entity::PLACEHOLDER,
		}
	}
	pub fn new_with_origin(payload: T, origin: Entity) -> OnRunLocal<T> {
		OnRunLocal { payload, origin }
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnRunGlobal<T = ()> {
	pub payload: T,
	pub origin: Entity,
	pub action: Entity,
}

impl<T> OnRunGlobal<T> {
	pub fn new(action: Entity, payload: T) -> Self {
		Self {
			payload,
			origin: action,
			action,
		}
	}
	pub fn new_with_origin(action: Entity, origin: Entity, payload: T) -> Self {
		Self {
			payload,
			origin,
			action,
		}
	}
}

impl<T: RunPayload> OnRun<T> {
	pub fn trigger_next(&self, mut commands: Commands, next_action: Entity) {
		commands.entity(next_action).trigger(OnRunLocal {
			payload: self.payload.clone(),
			origin: self.origin,
		});
	}
	pub fn trigger_next_with(
		&self,
		mut commands: Commands,
		next_action: Entity,
		next_payload: T,
	) {
		commands.entity(next_action).trigger(OnRunLocal {
			payload: next_payload,
			origin: self.origin,
		});
	}

	pub fn trigger_result(&self, mut commands: Commands, payload: T::Result) {
		commands.entity(self.action).trigger(OnResult {
			payload,
			origin: self.origin,
			action: self.action,
		});
	}
}

impl OnRun<()> {
	/// Usability helper, see [`OnRunLocal::new`].
	pub fn local() -> OnRunLocal { OnRunLocal::new(()) }
	/// Usability helper, see [`OnRunGlobal::new`].
	pub fn global(action: Entity) -> OnRunGlobal {
		OnRunGlobal::new(action, ())
	}
}

pub(crate) fn propagate_on_run_local<T: RunPayload>(
	ev: Trigger<OnRunLocal<T>>,
	mut commands: Commands,
	action_observers: Query<&ActionObservers>,
) {
	let action = ev.entity();
	if action == Entity::PLACEHOLDER {
		panic!("OnRunLocal must be triggered on an action entity");
	}
	if let Ok(observers) = action_observers.get(ev.entity()) {
		// OnRunLocal::new uses placeholder, replace with action entity
		let origin = if ev.origin == Entity::PLACEHOLDER {
			action
		} else {
			ev.origin
		};
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
pub(crate) fn propagate_on_run_global<T: RunPayload>(
	ev: Trigger<OnRunGlobal<T>>,
	mut commands: Commands,
	action_observers: Query<&ActionObservers>,
) {
	let action = ev.action;
	if let Ok(observers) = action_observers.get(action) {
		commands.trigger_targets(
			OnRun {
				payload: ev.payload.clone(),
				origin: ev.origin,
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
