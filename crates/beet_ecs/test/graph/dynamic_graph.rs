use beet_ecs::prelude::*;
use sweet::*;

#[sweet_test]
pub fn serde() -> Result<()> {
	let tree = WillyBehaviorTree::new(EmptyAction);
	let graph = tree.into_behavior_graph();
	let prefab1 = graph.into_prefab::<EcsNode>()?;
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
	let tree = WillyBehaviorTree::new(EmptyAction);
	let graph = tree.into_behavior_graph();
	expect(graph.into_prefab::<BadList>().map(|_| ())).to_be_err()?;
	Ok(())
}
