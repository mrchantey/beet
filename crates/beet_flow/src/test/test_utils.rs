use crate::prelude::*;
use bevy::prelude::*;
use sweet::*;

pub fn test_no_action_behavior_tree(world: &mut World) -> EntityTree {
	let entity = world
		.spawn(Running)
		.with_children(|parent| {
			parent.spawn_empty();
			parent.spawn_empty().with_children(|parent| {
				parent.spawn_empty();
			});
		})
		.id();
	EntityTree::new_with_world(entity, world)
}

type Func<T> = MockFunc<T, T, fn(T) -> T>;

pub fn observe_run_results(world: &mut World) -> Func<RunResult> {
	let func: Func<RunResult> = mock_func(|a| a);
	let func2 = func.clone();
	world.observe(move |on_result: Trigger<OnRunResult>| {
		func2.call(on_result.event().result());
	});
	func
}
