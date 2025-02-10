use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;



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

/// Add this to an entity to prevent the run result from bubbling up.
///
/// Any action that requires this needs to manually call OnChildResult
/// on the parent entity. For an example, see [`RepeatFlow`].
#[derive(Default, Component, Reflect)]
pub struct NoBubble;



/// Global observer to call OnRun for each action registered
/// on the action's *parent* entity. In this way it differes from
/// [propagate_request_to_observers].
pub fn propagate_response_to_observers<R: Request>(
	res: Trigger<OnResponse<R>>,
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
			let mut res = (*res).clone();
			res.action = parent;
			commands.trigger_targets(res, (*action_observers).clone());
		}
	}
}

#[action(bubble_result::<R>)]
#[derive(Debug, Component, Default, Clone, Copy, PartialEq, Reflect)]
pub struct BubbleUpFlow<R: Request>(PhantomData<R>);

fn bubble_result<R: Request>(
	trigger: Trigger<OnResponse<R>>,
	mut commands: Commands,
) {
	// let mut res = (*trigger).clone();
	commands.entity(trigger.action).trigger(trigger.clone());
	// trigger.on_result(commands);
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn bubbles_up() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();
		let counter = observe_triggers::<OnRunResult>(world);
		let mut child = Entity::PLACEHOLDER;
		let mut grandchild = Entity::PLACEHOLDER;

		let parent = world
			.spawn(BubbleUpFlow::<Run>::default())
			.with_children(|parent| {
				child = parent
					.spawn(BubbleUpFlow::<Run>::default())
					.with_children(|parent| {
						grandchild =
							parent.spawn(Run::fixed_response(Ok(()))).id();
					})
					.id();
			})
			.id();
		world.entity_mut(grandchild).flush_trigger(OnRun::default());

		expect(&counter).to_have_been_called_times(5);
		expect(&counter).to_have_returned_nth_with(
			0,
			&OnResponse::<Run>(ActionContext {
				payload: Ok(()),
				target: grandchild,
				action: grandchild,
			}),
		);
		expect(&counter).to_have_returned_nth_with(
			1,
			&OnResponse::<Run>(ActionContext {
				payload: Ok(()),
				target: grandchild,
				action: child,
			}),
		);
		expect(&counter).to_have_returned_nth_with(
			3,
			&OnResponse::<Run>(ActionContext {
				payload: Ok(()),
				target: grandchild,
				action: parent,
			}),
		);
	}
}
