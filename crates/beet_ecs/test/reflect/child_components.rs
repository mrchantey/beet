use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test(skip)]
fn spawns() -> Result<()> {
	let mut world = World::new();

	let agent = world.spawn_empty().id();
	let _root = ScoreSelector
		.child(EmptyAction)
		.into_beet_node()
		.with_type::<Score>() // not needed by happenstance but usually required
		.spawn(&mut world, agent)
		.value;
	// expect(&world).component(root)?.to_be(&Score::default())?;
	// expect(true).to_be_false()?;

	Ok(())
}
