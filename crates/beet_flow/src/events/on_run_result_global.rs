use crate::prelude::*;
use bevy::prelude::*;

/// Signifies a behavior has stopped running. This bubbles
/// up the tree until it reaches the root node or a [`StopBubble`].
#[derive(Debug, Event, Clone, Copy, PartialEq, Reflect)]
pub struct OnRunResultGlobal {
	pub result: RunResult,
	pub context: RunContext,
}

// impl Event for OnRunResultGlobal {
// 	type Traversal = &'static Parent;
// 	const AUTO_PROPAGATE: bool = true;
// }


impl OnRunResultGlobal {
	pub fn new(context: RunContext, result: RunResult) -> Self {
		Self { result, context }
	}
	/// Populate with [`RunResult::Success`]
	pub fn success(context: RunContext) -> Self {
		Self {
			context,
			result: RunResult::Success,
		}
	}
	/// Populate with [`RunResult::Failure`]
	pub fn failure(context: RunContext) -> Self {
		Self {
			context,
			result: RunResult::Failure,
		}
	}
	pub fn into_child_result(self, parent: Entity) -> OnChildResultGlobal {
		OnChildResultGlobal {
			context: RunContext {
				target: self.context.target,
				action: parent,
			},
			result: self.result,
		}
	}
}

#[derive(Debug, Event, Clone, Copy, PartialEq, Reflect)]
pub struct OnChildResultGlobal {
	pub result: RunResult,
	pub context: RunContext,
}
impl OnChildResultGlobal {
	pub fn into_run_result(self) -> OnRunResultGlobal {
		OnRunResultGlobal::new(self.context, self.result)
	}
}


/// When [`OnRunResult`] is triggered, propagate to parent with [`OnChildResult`].
/// We can't use bevy event propagation because that does not track the last entity
/// that called bubble, which is required for selectors.
pub fn bubble_run_result_global(
	trigger: Trigger<OnRunResultGlobal>,
	mut commands: Commands,
	// no_bubble: Query<(), With<NoBubble>>,
	action_map: Res<ActionMap>,
	parents: Query<&Parent>,
) {
	// we dont need this anymore?
	// if no_bubble.contains(trigger.context.action) {
	// 	return;
	// }

	if let Ok(parent) = parents.get(trigger.context.action) {
		let parent = parent.get();
		if let Some(action_observers) = action_map.action_observers.get(&parent)
		{
			commands.trigger_targets(
				trigger.into_child_result(parent),
				action_observers.clone(),
			);
		}
		// commands
		// 	.entity(parent)
		// 	.trigger(trigger.into_child_result(parent));
		// .trigger(trigger.into_child_result(parent));
	}
}

/// Simply convert an `OnChildResult` into an `OnRunResult`.
#[derive(Debug, GlobalAction, Clone, Copy, PartialEq, Reflect)]
#[observers(bubble_result)]
pub struct BubbleUpFlow;

pub fn bubble_result(
	trigger: Trigger<OnChildResultGlobal>,
	mut commands: Commands,
) {
	commands
		.entity(trigger.context.action)
		.trigger(trigger.into_run_result());
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn bubbles_up() {
		let mut app = App::new();
		app.add_plugins(on_run_global_plugin);
		let world = app.world_mut();
		let counter = observe_triggers::<OnRunResultGlobal>(world);
		let mut child = Entity::PLACEHOLDER;
		let mut grandchild = Entity::PLACEHOLDER;

		let parent = world
			.spawn(BubbleUpFlow)
			.with_children(|parent| {
				child = parent
					.spawn(BubbleUpFlow)
					.with_children(|parent| {
						grandchild =
							parent.spawn(EndOnRunGlobal::success()).id();
					})
					.id();
			})
			.id();
		world
			.entity_mut(grandchild)
			.flush_trigger(OnRunGlobal::default());

		expect(&counter).to_have_been_called_times(3);
		expect(&counter).to_have_returned_nth_with(0, &OnRunResultGlobal {
			result: RunResult::Success,
			context: RunContext {
				target: grandchild,
				action: grandchild,
			},
		});
		expect(&counter).to_have_returned_nth_with(1, &OnRunResultGlobal {
			result: RunResult::Success,
			context: RunContext {
				target: grandchild,
				action: child,
			},
		});
		expect(&counter).to_have_returned_nth_with(2, &OnRunResultGlobal {
			result: RunResult::Success,
			context: RunContext {
				target: grandchild,
				action: parent,
			},
		});
	}
	#[test]
	fn stop_bubble() {
		let mut app = App::new();
		app.add_plugins(on_run_global_plugin);
		let world = app.world_mut();
		let counter = observe_triggers::<OnRunResultGlobal>(world);

		let mut child = Entity::PLACEHOLDER;
		let mut grandchild = Entity::PLACEHOLDER;

		let _parent =
			world.spawn_empty().with_child(()).with_children(|parent| {
				child = parent
					.spawn(NoBubble::default())
					.with_children(|parent| {
						grandchild =
							parent.spawn(EndOnRunGlobal::success()).id();
					})
					.id();
			});

		world
			.entity_mut(grandchild)
			.flush_trigger(OnRunGlobal::default());

		// only child and grandchild called
		expect(&counter).to_have_been_called_times(2);

		world.entity_mut(child).remove::<NoBubble>();
		world
			.entity_mut(grandchild)
			.flush_trigger(OnRunResultGlobal::success(
				RunContext::with_action(child),
			));
		// it was removed so all called
		expect(&counter).to_have_been_called_times(5);
	}
}
