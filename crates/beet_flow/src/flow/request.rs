use crate::prelude::*;
use bevy::prelude::*;

pub fn request_plugin<T: Request>(app: &mut App) {
	app.add_observer(propagate_request_to_observers::<T>);
	app.add_observer(propagate_response_to_parent_observers::<T::Res>);
}

pub trait Request: ActionPayload {
	type Res: Response<Req = Self>;
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
pub fn propagate_request_to_observers<R: Request>(
	req: Trigger<ActionContext<R>>,
	mut commands: Commands,
	action_observers: Query<&ActionObservers>,
	action_observer_markers: Query<(), With<ActionObserverMarker>>,
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

	let target = if req.origin == Entity::PLACEHOLDER {
		action
	} else {
		req.origin
	};


	if let Ok(actions) = action_observers.get(action) {
		let mut req = (*req).clone();
		req.action = action;
		req.origin = target;
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
		trigger: Trigger<ActionContext<Run>>,
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
			.flush_trigger(ActionContext::new(Run))
			.id();

		expect(app.world().get::<TriggerCount>(entity).unwrap().0).to_be(1);
	}
	#[test]
	fn explicit_action() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());

		let entity = app.world_mut().spawn(TriggerCount::default()).id();
		app.world_mut()
			.flush_trigger(ActionContext::new_with_action(entity, Run));

		expect(app.world().get::<TriggerCount>(entity).unwrap().0).to_be(1);
	}
}
