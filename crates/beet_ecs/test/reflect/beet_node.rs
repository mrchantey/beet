use beet_ecs::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use sweet::*;

#[derive(Component, Reflect)]
pub struct Foobar;

#[sweet_test]
fn works() -> Result<()> {
	let _node = BeetNode::new(EmptyAction);
	let _node2 = BeetNode::new((EmptyAction, Foobar, ConstantScore::default()));
	let node = EmptyAction.child2((EmptyAction, ConstantScore::default()));

	let _prefab = node.into_prefab::<EcsNode>();

	Ok(())
}
