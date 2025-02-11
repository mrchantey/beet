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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct OnRun<T = ()> {
	pub payload: T,
	pub origin: Entity,
	pub action: Entity,
	pub prev_action: Entity,
}

impl<T: RunPayload> OnRun<T> {
	pub fn new_local(payload: T) -> Self {
		Self {
			payload,
			origin: Entity::PLACEHOLDER,
			// inferred action, it will be set by [run_action_observers]
			action: Entity::PLACEHOLDER,
			prev_action: Entity::PLACEHOLDER,
		}
	}
	pub fn new_global(action: Entity, payload: T) -> Self {
		Self {
			payload,
			origin: action,
			action,
			prev_action: Entity::PLACEHOLDER,
		}
	}
	pub fn trigger_next(&self, mut commands: Commands, next_action: Entity) {
		commands.entity(next_action).trigger(Self {
			payload: self.payload.clone(),
			origin: self.origin,
			action: next_action,
			prev_action: self.action,
		});
	}
	pub fn trigger_next_with(
		&self,
		mut commands: Commands,
		next_action: Entity,
		next_payload: T,
	) {
		commands.entity(next_action).trigger(Self {
			payload: next_payload,
			action: next_action,
			origin: self.origin,
			prev_action: self.action,
		});
	}

	pub fn trigger_result(&self, mut commands: Commands, payload: T::Result) {
		commands.entity(self.action).trigger(OnResult {
			payload,
			origin: self.origin,
			action: self.action,
			prev_action: self.prev_action,
		});
	}
}

impl OnRun<()> {
	/// Usability helper, see [`Self::new_local`].
	pub fn local() -> Self { Self::new_local(()) }
	/// Usability helper, see [`Self::new_global`].
	pub fn global(action: Entity) -> Self { Self::new_global(action, ()) }
}
/// Global observer to call OnRun for each action registered
/// on the action entity.
///
/// # Panics
/// If the trigger does specify an action, usually because
/// `OnRun` was called directly without `with_target`.
///
/// Unlike [propagate_response_to_parent_observers], this will trigger
/// for the action observerse directly
pub fn run_action_observers<T: RunPayload>(
	ev: Trigger<OnRun<T>>,
	mut commands: Commands,
	action_observers: Query<&ActionObservers>,
	action_observer_markers: Query<(), With<ActionObserverMarker>>,
) {
	if action_observer_markers.contains(ev.entity()) {
		return;
	}

	let action = if ev.action == Entity::PLACEHOLDER {
		let trigger_entity = ev.entity();
		if trigger_entity == Entity::PLACEHOLDER {
			panic!("{}", expect_action::to_specify_action(&ev));
		}
		trigger_entity
	} else {
		ev.action
	};

	let origin = if ev.origin == Entity::PLACEHOLDER {
		action
	} else {
		ev.origin
	};


	if let Ok(actions) = action_observers.get(action) {
		let mut on_run = (*ev).clone();
		on_run.action = action;
		on_run.origin = origin;
		// println!("run_action_observers: {:?}\nactions: {:?}", on_run, actions);
		commands.trigger_targets(on_run, (**actions).clone());
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
