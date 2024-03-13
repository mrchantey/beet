use beet_ecs::prelude::*;
use bevy_ecs::prelude::*;
use sweet::*;

#[sweet_test(skip)]
fn spawns() -> Result<()> {
	let mut world = World::new();

	let agent = world.spawn_empty().id();
	let _root = *UtilitySelector
		.child(EmptyAction)
		.into_beet_node()
		.with_type::<Score>() // not needed by happenstance but usually required
		.spawn::<EcsNode>(&mut world, agent)
		.root()
		.unwrap();
	// expect(&world).component(root)?.to_be(&Score::default())?;
	// expect(true).to_be_false()?;

	Ok(())
}
