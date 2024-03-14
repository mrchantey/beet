use super::*;
use beet_ecs::prelude::*;
use bevy_app::App;
use sweet::*;


#[sweet_test]
pub fn default_components() -> Result<()> {
	let mut app = App::new();
	let target = app.world.spawn_empty().id();
	let actions = test_constant_behavior_tree();
	let entities = actions.spawn(&mut app, target);
	let entity = *entities.root().unwrap();

	expect(&app).to_have_component::<SetOnStart<Score>>(entity)?;
	expect(&app).to_have_component::<TargetAgent>(entity)?;
	expect(&app).to_have_component::<RunTimer>(entity)?;
	expect(&app).to_have_component::<Score>(entity)?;


	Ok(())
}

#[sweet_test]
pub fn sync_system() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());

	let target = app.world.spawn_empty().id();
	let actions = test_constant_behavior_tree();
	let entities = actions.spawn(&mut app, target);
	let entity = *entities.root().unwrap();


	app.world
		.entity_mut(entity)
		.insert(SetOnStart::new(Score::Pass));

	expect(&app).component(entity)?.to_be(&Score::Fail)?;
	app.update();
	expect(&app).component(entity)?.to_be(&Score::Pass)?;

	Ok(())
}
