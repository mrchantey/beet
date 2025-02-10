use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


pub trait Response: ActionPayload {
	type Req: Request<Res = Self>;
}

/// Add this to an entity to prevent the run result from bubbling up.
///
/// Any action that requires this needs to manually call OnChildResult
/// on the parent entity. For an example, see [`RepeatFlow`].
#[derive(Default, Component, Reflect)]
pub struct NoBubble;



/// Global observer to pass an action up to all *parent observers*,
/// so they may handle the response.
/// 
/// Unlike [propagate_request_to_observers], this is called on parent
/// observers.
pub fn propagate_response_to_parent_observers<R: Response>(
	res: Trigger<ActionContext<R>>,
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
pub struct BubbleUpFlow<R: Response>(PhantomData<R>);


/// An action is usually triggered
fn bubble_result<R: Response>(
	trigger: Trigger<ActionContext<R>>,
	mut commands: Commands,
) {
	commands.entity(trigger.action).trigger(trigger.clone());
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
		let counter = observe_triggers::<ActionContext<RunResult>>(world);
		let mut child = Entity::PLACEHOLDER;
		let mut grandchild = Entity::PLACEHOLDER;

		let parent = world
			.spawn(BubbleUpFlow::<RunResult>::default())
			.with_children(|parent| {
				child = parent
					.spawn(BubbleUpFlow::<RunResult>::default())
					.with_children(|parent| {
						grandchild =
							parent.spawn(RespondWith(RunResult::Success)).id();
					})
					.id();
			})
			.id();
		world.entity_mut(grandchild).flush_trigger(Run.trigger());

		expect(&counter).to_have_been_called_times(5);
		expect(&counter).to_have_returned_nth_with(0, &ActionContext {
			payload: RunResult::Success,
			origin: grandchild,
			action: grandchild,
		});
		expect(&counter).to_have_returned_nth_with(1, &ActionContext {
			payload: RunResult::Success,
			origin: grandchild,
			action: child,
		});
		expect(&counter).to_have_returned_nth_with(3, &ActionContext {
			payload: RunResult::Success,
			origin: grandchild,
			action: parent,
		});
	}
}
