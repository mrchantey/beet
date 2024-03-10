use beet_core::prelude::*;
use bevy_app::App;
use bevy_math::Vec3;
use bevy_transform::prelude::*;
use bevy_utils::default;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();

	app.add_plugins(SteeringPlugin::default());
	app.insert_time();

	let velocity_entity = app
		.world
		.spawn((TransformBundle::default(), ForceBundle {
			velocity: Velocity(Vec3::new(1., 0., 0.)),
			..default()
		}))
		.id();
	let force_entity = app
		.world
		.spawn((TransformBundle::default(), ForceBundle {
			force: Force(Vec3::new(1., 0., 0.)),
			..default()
		}))
		.id();
	let impulse_entity = app
		.world
		.spawn((TransformBundle::default(), ForceBundle {
			impulse: Impulse(Vec3::new(1., 0., 0.)),
			..default()
		}))
		.id();

	let mass_entity = app
		.world
		.spawn((TransformBundle::default(), ForceBundle {
			mass: Mass(2.),
			impulse: Impulse(Vec3::new(1., 0., 0.)),
			..default()
		}))
		.id();



	app.update_with_secs(1);

	expect(&app)
		.component::<Transform>(velocity_entity)?
		.map(|t| t.translation)
		.to_be(Vec3::new(1., 0., 0.))?;
	expect(&app)
		.component::<Transform>(force_entity)?
		.map(|t| t.translation)
		.to_be(Vec3::new(1., 0., 0.))?;
	expect(&app)
		.component::<Transform>(impulse_entity)?
		.map(|t| t.translation)
		.to_be(Vec3::new(1., 0., 0.))?;
	expect(&app)// impulses are cleared each frame
		.component(impulse_entity)?
		.to_be(&Impulse(Vec3::ZERO))?;
	expect(&app)
		.component::<Transform>(mass_entity)?
		.map(|t| t.translation)
		.to_be(Vec3::new(0.5, 0., 0.))?;

	app.update_with_secs(1);

	expect(&app)
		.component::<Transform>(velocity_entity)?
		.map(|t| t.translation)
		.to_be(Vec3::new(2., 0., 0.))?;
	expect(&app)
		.component::<Transform>(force_entity)?
		.map(|t| t.translation)
		.to_be(Vec3::new(3., 0., 0.))?;
	expect(&app)
		.component::<Transform>(impulse_entity)?
		.map(|t| t.translation)
		.to_be(Vec3::new(2., 0., 0.))?;
	expect(&app)
		.component::<Transform>(mass_entity)?
		.map(|t| t.translation)
		.to_be(Vec3::new(1., 0., 0.))?;


	Ok(())
}
