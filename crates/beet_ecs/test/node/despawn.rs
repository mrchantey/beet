use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	app.add_systems(PostUpdate, despawn_graph_on_agent_removed);

	let target = app.world.spawn_empty().id();

	InsertOnRun(RunResult::Success)
		.into_beet_node()
		.spawn(&mut app.world, target);

	expect(app.world.entities().len()).to_be(2)?;
	app.world.despawn(target);
	expect(app.world.entities().len()).to_be(1)?;
	app.update();
	expect(app.world.entities().len()).to_be(0)?;


	Ok(())
}
