use super::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;


#[sweet_test]
pub fn default_components() -> Result<()> {
	let mut app = App::new();
	let target = app.world.spawn_empty().id();
	let actions = test_constant_behavior_tree();
	let root = actions.spawn(&mut app.world, target).value;

	expect(&app).to_have_component::<SetOnStart<Score>>(root)?;
	expect(&app).to_have_component::<TargetAgent>(root)?;
	expect(&app).to_have_component::<RunTimer>(root)?;
	expect(&app).to_have_component::<Score>(root)?;


	Ok(())
}

#[sweet_test]
pub fn sync_system() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());

	let target = app.world.spawn_empty().id();
	let actions = test_constant_behavior_tree();
	let root = actions.spawn(&mut app.world, target).value;

	app.world.entity_mut(root).insert(SetOnStart(Score::Pass));

	expect(&app).component(root)?.to_be(&Score::Fail)?;
	app.update();
	expect(&app).component(root)?.to_be(&Score::Pass)?;

	Ok(())
}
