use beet_core::prelude::*;
use beet_ecs::action::ActionPlugin;
use beet_ecs::graph::BehaviorTree;
use bevy_app::App;
use bevy_math::Vec3;
use bevy_transform::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();

	app.add_plugins((
		SteeringPlugin::default(),
		ActionPlugin::<CoreNode, _>::default(),
	))
	.insert_time();

	let agent = app
		.world
		.spawn((
			TransformBundle::default(),
			ForceBundle::default(),
			SteerBundle::default().with_target(Vec3::new(1.0, 0., 0.)),
		))
		.id();

	let tree = BehaviorTree::<CoreNode>::new(Seek);
	tree.spawn(&mut app, agent);

	app.update();
	app.update_with_secs(1);

	expect(&app)
		.component::<Transform>(agent)?
		.map(|t| t.translation)
		.to_be(Vec3::new(0.2, 0., 0.))?;


	Ok(())
}
