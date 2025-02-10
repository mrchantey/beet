use crate::prelude::*;
use bevy::prelude::*;


pub fn request_plugin<T: Request>(app: &mut App) {
	app.add_observer(propagate_request_to_observers::<T>);
}


#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event, Deref, DerefMut,
)]
pub struct OnRequest<T: Request>(pub ActionContext<T>);

impl<T: Request> OnRequest<T> {
	pub fn new(payload: T) -> Self {
		Self(ActionContext {
			payload,
			target: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		})
	}
	pub fn new_with_action(action: Entity, payload: T) -> Self {
		Self(ActionContext {
			payload,
			target: action,
			action,
		})
	}
	pub fn new_with_action_and_target(
		action: Entity,
		target: Entity,
		payload: T,
	) -> Self {
		Self(ActionContext {
			payload,
			target,
			action,
		})
	}
	pub fn into_response(&self, payload: T::Res) -> OnResponse<T> {
		let cx = ActionContext {
			payload,
			target: self.target,
			action: self.action,
		};
		OnResponse(cx)
	}
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event, Deref, DerefMut,
)]
pub struct OnResponse<T: Request>(pub ActionContext<T::Res>);

impl<T: Request> OnResponse<T> {
	pub fn new(payload: T::Res) -> Self {
		Self(ActionContext {
			payload,
			target: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		})
	}
	pub fn new_with_action(action: Entity, payload: T::Res) -> Self {
		Self(ActionContext {
			payload,
			target: action,
			action,
		})
	}
	pub fn new_with_action_and_target(
		payload: T::Res,
		action: Entity,
		target: Entity,
	) -> Self {
		Self(ActionContext {
			payload,
			target,
			action,
		})
	}
}




#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Event)]
pub struct ActionContext<T> {
	pub payload: T,
	pub target: Entity,
	pub action: Entity,
}

impl<T> ActionContext<T> {
	pub fn placeholder(payload: T) -> Self {
		Self {
			payload,
			target: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		}
	}
}

pub trait Request: 'static + Send + Sync + Clone {
	type Res: 'static + Send + Sync + Clone;
}


/// Global observer to call OnRun for each action registered
/// on the action entity.
///
/// # Panics
/// If the trigger does specify an action, usually because
/// `OnRun` was called directly without `with_target`
pub fn propagate_request_to_observers<R: Request>(
	req: Trigger<OnRequest<R>>,
	action_observer_markers: Query<&ActionObserverMarker>,
	action_observers: Query<&ActionObservers>,
	mut commands: Commands,
) {
	if action_observer_markers.contains(req.entity()) {
		return;
	}

	let action = if req.action == Entity::PLACEHOLDER {
		let trigger_entity = req.entity();
		if trigger_entity == Entity::PLACEHOLDER {
			panic!("{}", expect_action::to_specify_action(req.action));
		}
		trigger_entity
	} else {
		req.action
	};

	let target = if req.target == Entity::PLACEHOLDER {
		action
	} else {
		req.target
	};


	if let Ok(actions) = action_observers.get(action) {
		let mut req = (*req).clone();
		req.action = action;
		req.target = target;
		commands.trigger_targets(req, (**actions).clone());
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
		trigger: Trigger<OnRequest<Run>>,
		mut query: Query<&mut TriggerCount>,
	) {
		query.get_mut(trigger.action).unwrap().as_mut().0 += 1;
	}

	#[test]
	fn inferred_action() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());

		let entity = app
			.world_mut()
			.spawn(TriggerCount::default())
			.flush_trigger(OnRequest::new(Run))
			.id();

		expect(app.world().get::<TriggerCount>(entity).unwrap().0).to_be(1);
	}
	#[test]
	fn explicit_action() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());

		let entity = app.world_mut().spawn(TriggerCount::default()).id();
		app.world_mut()
			.flush_trigger(OnRequest::new_with_action(entity, Run));

		expect(app.world().get::<TriggerCount>(entity).unwrap().0).to_be(1);
	}
}
