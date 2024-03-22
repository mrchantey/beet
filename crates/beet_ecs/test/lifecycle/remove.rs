use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();

	app.add_plugins(BeetSystemsPlugin::<EcsNode, _>::default());


	let target = app.world.spawn_empty().id();
	InsertOnRun(RunResult::Success)
		.into_beet_node()
		.spawn(&mut app.world, target);

	expect(app.world.entities().len()).to_be(2)?;
	app.update();
	app.world.despawn(target);

	expect(app.world.entities().len()).to_be(1)?;
	app.update();
	expect(app.world.entities().len()).to_be(0)?;

	Ok(())
}
