use beet_ecs::prelude::*;
use bevy_core::Name;
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
fn serde_bytes() -> Result<()> {
	let prefab1 = BehaviorPrefab::<EcsNode>::from_graph(EmptyAction)?;
	let bytes1 = bincode::serialize(&prefab1)?;
	let prefab2: BehaviorPrefab<EcsNode> = bincode::deserialize(&bytes1)?;
	let bytes2 = bincode::serialize(&prefab2)?;
	expect(bytes1).to_be(bytes2)?;
	Ok(())
}
#[sweet_test]
/// these are to be in sync with [`BehaviorPrefab::append_type_registry`]
fn serde_types() -> Result<()> {
	let prefab1 = BehaviorPrefab::<EcsNode>::from_graph(
		EmptyAction.child(ConstantScore::default()),
	)?;
	let bytes1 = bincode::serialize(&prefab1)?;
	let prefab2: BehaviorPrefab<EcsNode> = bincode::deserialize(&bytes1)?;
	let mut world = World::new();
	let target = world.spawn_empty().id();
	let root = prefab2.spawn(&mut world, Some(target))?;
	let child = world.entity(root).get::<Edges>().unwrap()[0];
	expect(&world).component(child)?.to_be(&Score::default())?;

	expect(&world).to_have_component::<Name>(root)?;
	expect(&world).to_have_component::<Edges>(root)?;
	expect(&world).to_have_component::<Running>(root)?;
	expect(&world).to_have_component::<RunTimer>(root)?;
	expect(&world).to_have_component::<BehaviorGraphRoot>(root)?;


	Ok(())
}
#[derive(Debug, Clone)]
struct BadList;
impl ActionTypes for BadList {
	fn register(_: &mut bevy_reflect::TypeRegistry) {}
}

#[sweet_test]
fn fails() -> Result<()> {
	expect(BehaviorPrefab::<BadList>::from_graph(EmptyAction).map(|_| ()))
		.to_be_err()?;
	Ok(())
}
#[sweet_test]
fn spawns() -> Result<()> {
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
	expect(&world).component(root)?.to_be(&Score::default())?;

	Ok(())
}
