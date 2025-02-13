use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


/// Tags: [ControlFlow](ActionTag::ControlFlow)
///
/// If any child triggers a result, bubble it up to the parent.
#[action(bubble_result::<T>)]
#[derive(Debug, Component, Clone, Copy, PartialEq, Reflect)]
pub struct BubbleResult<T: ResultPayload = RunResult>(PhantomData<T>);

impl Default for BubbleResult {
	fn default() -> Self { Self(PhantomData) }
}


/// An action is usually triggered
fn bubble_result<T: ResultPayload>(
	ev: Trigger<OnChildResult<T>>,
	commands: Commands,
) {
	ev.trigger_bubble(commands);
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
		let counter = observe_triggers::<OnResultAction>(world);
		let mut child = Entity::PLACEHOLDER;
		let mut grandchild = Entity::PLACEHOLDER;

		let parent = world
			.spawn(BubbleResult::default())
			.with_children(|parent| {
				child = parent
					.spawn(BubbleResult::default())
					.with_children(|parent| {
						grandchild =
							parent.spawn(ReturnWith(RunResult::Success)).id();
					})
					.id();
			})
			.id();
		world.entity_mut(grandchild).flush_trigger(OnRun::local());

		expect(&counter).to_have_been_called_times(3);
		expect(&counter).to_have_returned_nth_with(
			0,
			&OnResultAction::new(grandchild, grandchild, RunResult::Success),
		);
		expect(&counter).to_have_returned_nth_with(
			1,
			&OnResultAction::new(child, grandchild, RunResult::Success),
		);
		expect(&counter).to_have_returned_nth_with(
			2,
			&OnResultAction::new(parent, grandchild, RunResult::Success),
		);
	}
}
