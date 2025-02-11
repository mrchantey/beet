use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


#[action(bubble_result::<T>)]
#[derive(Debug, Component, Clone, Copy, PartialEq, Reflect)]
pub struct BubbleUpFlow<T: ResultPayload = RunResult>(PhantomData<T>);

impl Default for BubbleUpFlow {
	fn default() -> Self { Self(PhantomData) }
}


/// An action is usually triggered
fn bubble_result<T: ResultPayload>(
	trig: Trigger<OnResult<T>>,
	commands: Commands,
) {
	trig.trigger_bubble(commands);
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
		let counter = observe_triggers::<OnResult>(world);
		let mut child = Entity::PLACEHOLDER;
		let mut grandchild = Entity::PLACEHOLDER;

		let parent = world
			.spawn(BubbleUpFlow::default())
			.with_children(|parent| {
				child = parent
					.spawn(BubbleUpFlow::default())
					.with_children(|parent| {
						grandchild =
							parent.spawn(RespondWith(RunResult::Success)).id();
					})
					.id();
			})
			.id();
		world.entity_mut(grandchild).flush_trigger(OnRun::local());

		expect(&counter).to_have_been_called_times(5);
		expect(&counter).to_have_returned_nth_with(0, &OnResult {
			payload: RunResult::Success,
			origin: grandchild,
			action: grandchild,
			prev_action: Entity::PLACEHOLDER,
		});
		expect(&counter).to_have_returned_nth_with(1, &OnResult {
			payload: RunResult::Success,
			origin: grandchild,
			action: child,
			prev_action: grandchild,
		});
		expect(&counter).to_have_returned_nth_with(3, &OnResult {
			payload: RunResult::Success,
			origin: grandchild,
			action: parent,
			prev_action: child,
		});
	}
}
