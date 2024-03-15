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

	let _prefab = node.into_prefab();

	Ok(())
}
