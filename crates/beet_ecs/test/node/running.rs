// use bevy_ecs::prelude::*;
use beet_ecs::prelude::*;
use bevy_app::App;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	// expect(true).to_be_false()?;


	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());

	let target = app.world.spawn_empty().id();

	let action_graph = BehaviorTree::<EcsNode>::new(SetRunResult::default())
		.into_action_graph();

	let entity_graph = action_graph.spawn(&mut app.world, target);
	let root = *entity_graph.root().unwrap();

	expect(&app).to_have_component::<Running>(root)?;
	// add `RunResult`, remove `Running`
	app.update();
	expect(&app).not().to_have_component::<Running>(root)?;
	expect(&app).to_have_component::<RunResult>(root)?;
	// remove `Running`
	app.update();
	// remove `RunResult`
	expect(&app).not().to_have_component::<Running>(root)?;
	expect(&app).not().to_have_component::<RunResult>(root)?;

	Ok(())
}
