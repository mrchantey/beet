use beet_core::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;


#[sweet_test]
fn algo() -> Result<()> {
	let impulse = seek_impulse(
		&Vec3::default(),
		&Velocity::default(),
		&Vec3::new(1., 0., 0.),
		MaxSpeed::default(),
		MaxForce::default(),
		None,
	);
	expect(*impulse).to_be(Vec3::new(*MaxForce::default(), 0., 0.))?;

	Ok(())
}

#[sweet_test]
fn action() -> Result<()> {
	let mut app = App::new();

	app.add_plugins((
		SteeringPlugin::default(),
		BeetSystemsPlugin::<CoreNode, _>::default(),
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

	Seek.into_beet_node().spawn(&mut app.world, agent);

	app.update();
	app.update_with_secs(1);

	expect(&app)
		.component::<Transform>(agent)?
		.map(|t| t.translation)
		.to_be(Vec3::new(0.2, 0., 0.))?;


	Ok(())
}
