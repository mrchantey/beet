use beet_ecs::prelude::*;
use bevy_app::App;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());
	app.insert_time();

	let tree = SucceedInDuration::default();
	let instance = EntityGraph::spawn_no_target(&mut app, tree);
	let root = *instance.root().unwrap();

	expect(&app).to_have_component::<Running>(root)?;

	app.update_with_secs(2);

	expect(&app).component(root)?.to_be(&RunResult::Success)?;
	expect(&app).not().to_have_component::<Running>(root)?;

	Ok(())
}
