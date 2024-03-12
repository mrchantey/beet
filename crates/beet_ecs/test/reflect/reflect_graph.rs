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
	let prefab1 = BehaviorPrefab::<EcsNode>::from_graph(EmptyAction)?;
	let bytes1 = bincode::serialize(&prefab1)?;
	let prefab2: BehaviorPrefab<EcsNode> = bincode::deserialize(&bytes1)?;
	let bytes2 = bincode::serialize(&prefab2)?;
	expect(bytes1).to_be(bytes2)?;
	Ok(())
}
#[derive(Debug, Clone)]
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
	let prefab = BehaviorPrefab::<EcsNode>::from_graph(ConstantScore::new(
		Score::Weight(0.5),
	))?;

	let mut world = World::new();

	let agent = world.spawn_empty().id();

	let root = prefab.spawn(&mut world, Some(agent))?;

	expect(&world).to_have_entity(root)?;
	expect(&world).component::<AgentMarker>(agent)?;
	expect(&world).component(root)?.to_be(&TargetAgent(agent))?;
	expect(&world)
		.component(root)?
		.to_be(&ConstantScore(Score::Weight(0.5)))?;

	// test shared component
	expect(&world).component(root)?.to_be(&Score::Weight(0.5))?;

	Ok(())
}
