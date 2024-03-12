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
