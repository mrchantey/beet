use beet_ecs::prelude::*;
use bevy_app::App;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();

	app.add_plugins(ActionPlugin::<EcsNode, _>::default());


	let target = app.world.spawn_empty().id();
	SetRunResult::success()
		.into_beet_node()
		.spawn(&mut app, target);

	expect(app.world.entities().len()).to_be(2)?;
	app.update();
	app.world.despawn(target);

	expect(app.world.entities().len()).to_be(1)?;
	app.update();
	expect(app.world.entities().len()).to_be(0)?;

	Ok(())
}
