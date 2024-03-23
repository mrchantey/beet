use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[derive(Component, Reflect)]
pub struct Foobar;

#[sweet_test]
fn works() -> Result<()> {
	let _node = BeetNode::new(EmptyAction);
	let _node2 =
		BeetNode::new((EmptyAction, Foobar, SetOnStart::<Score>::default()));
	let node = EmptyAction.child(
		(EmptyAction, SetOnStart::<Score>::default()).child(EmptyAction),
	);

	let _val = node.into_graph::<EcsNode>();

	Ok(())
}

#[sweet_test]
fn into() -> Result<()> {
	fn foo<M>(_val: impl IntoBeetNode<M>) {}

	let _ = foo(EmptyAction.child(EmptyAction));
	let _ = foo(EmptyAction
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
fn spawns() -> Result<()> {
	let mut world = World::new();

	let agent = world.spawn_empty().id();

	let root = (Score::default(), SetOnStart(Score::Weight(0.5)))
		.into_beet_node()
		.with_type::<Score>() // not needed by happenstance but usually required
		.spawn(&mut world, agent)
		.value;

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
