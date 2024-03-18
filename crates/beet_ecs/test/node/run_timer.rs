use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());
	app.insert_time();

	let root = EmptyAction
		.into_beet_node()
		.spawn_no_target(&mut app.world)
		.value;

	app.update_with_secs(1);

	let timer = app.world.get::<RunTimer>(root).unwrap();
	expect(timer.last_started.elapsed_secs()).to_be_close_to(1.0)?;
	expect(timer.last_stopped.elapsed_secs()).to_be_close_to(1.0)?;

	app.world.entity_mut(root).remove::<Running>();
	app.update_with_secs(1);

	let timer = app.world.get::<RunTimer>(root).unwrap();
	expect(timer.last_started.elapsed_secs()).to_be_close_to(2.0)?;
	expect(timer.last_stopped.elapsed_secs()).to_be_close_to(1.0)?;

	Ok(())
}
