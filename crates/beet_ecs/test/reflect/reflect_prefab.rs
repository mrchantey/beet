use beet_ecs::prelude::*;
use bevy_ecs::prelude::*;
use sweet::*;

#[sweet_test]
fn into() -> Result<()> {
	fn foo<M>(_val: impl IntoBeetNode<M>) {}

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

// #[derive(Debug, Clone)]
// struct BadList;
// impl ActionTypes for BadList {
// 	fn register(_: &mut bevy_reflect::TypeRegistry) {}
// }

// #[sweet_test(skip)]
// fn fails() -> Result<()> {
// 	expect(EmptyAction.into_prefab().map(|_| ())).to_be_err()?;
// 	Ok(())
// }
#[sweet_test]
fn spawns() -> Result<()> {
	let mut world = World::new();

	let agent = world.spawn_empty().id();

	let root = *(Score::default(), SetOnStart(Score::Weight(0.5)))
		.into_beet_node()
		.with_type::<Score>() // not needed by happenstance but usually required
		.spawn(&mut world, agent)
		.root()
		.unwrap();

	expect(&world).to_have_entity(root)?;
	expect(&world).component::<AgentMarker>(agent)?;
	expect(&world).component(root)?.to_be(&TargetAgent(agent))?;
	expect(&world)
		.component(root)?
		.to_be(&SetOnStart(Score::Weight(0.5)))?;

	// test shared component
	expect(&world).component(root)?.to_be(&Score::default())?;

	Ok(())
}
