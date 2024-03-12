use beet_ecs::prelude::*;
use bevy_ecs::prelude::*;
use sweet::*;

#[sweet_test]
fn into() -> Result<()> {
	fn foo<M>(_val: impl IntoBehaviorGraph<M>) {}

	let _prefab = foo(EmptyAction.child(EmptyAction));
	let _prefab = foo(EmptyAction
		.child((EmptyAction, EmptyAction))
		.child(EmptyAction)
		.child(
			(EmptyAction, EmptyAction)
				.child(EmptyAction)
				.child(EmptyAction),
		));


	Ok(())
}

#[sweet_test]
pub fn serde() -> Result<()> {
	let prefab1 = BehaviorGraphPrefab::<EcsNode>::from_graph(EmptyAction)?;
	let bytes1 = bincode::serialize(&prefab1)?;
	let prefab2: BehaviorGraphPrefab<EcsNode> = bincode::deserialize(&bytes1)?;
	let bytes2 = bincode::serialize(&prefab2)?;
	expect(bytes1).to_be(bytes2)?;
	Ok(())
}
#[derive(Debug)]
struct BadList;
impl ActionTypes for BadList {
	fn register(_: &mut bevy_reflect::TypeRegistry) {}
}

#[sweet_test]
pub fn fails() -> Result<()> {
	let tree = BehaviorTree::new(EmptyAction);
	let graph = tree.into_behavior_graph();
	expect(graph.into_prefab::<BadList>().map(|_| ())).to_be_err()?;
	Ok(())
}
#[sweet_test]
pub fn spawns() -> Result<()> {
	let prefab = BehaviorGraphPrefab::<EcsNode>::from_graph(
		BehaviorTree::new(EmptyAction),
	)?;

	let mut world = World::new();

	let agent = world.spawn_empty().id();

	let result = prefab.spawn(&mut world, Some(agent))?;

	expect(&world).to_have_entity(result)?;
	// expect(&world).component(root)?;

	// let tree = BehaviorTree::new(EmptyAction);
	// let graph = tree.into_behavior_graph();
	// expect(graph.into_prefab::<BadList>().map(|_| ())).to_be_err()?;
	Ok(())
}
