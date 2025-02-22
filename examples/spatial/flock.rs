use beet::examples::scenes;
use beet::prelude::*;
use bevy::prelude::*;
use sweet::prelude::*;

#[rustfmt::skip]
pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.init_resource::<RandomSource>()
		.init_resource::<WrapAround>()
		// .add_plugins(DebugGroupSteerPlugin::<GroupSteerAgent>::default())
		.add_systems(Startup, (
			scenes::camera_2d, 
			scenes::space_scene, 
			setup
		))
		.run();
}


const SCALE: f32 = 100.;

fn setup(
	mut commands: Commands,
	mut rand: ResMut<RandomSource>,
	asset_server: Res<AssetServer>,
) {
	let ship = asset_server.load("spaceship_pack/ship_2.png");

	for _ in 0..300 {
		let position = Vec3::random_in_sphere(&mut rand).with_z(0.) * 500.;
		commands.spawn((
			Name::new("Spaceship"),
			Sprite {
				image: ship.clone(),
				..default()
			},
			Transform::from_translation(position).with_scale(Vec3::splat(0.5)),
			RotateToVelocity2d,
			ForceBundle::default(),
			SteerBundle::default().scaled_dist(SCALE),
			VelocityScalar(Vec3::new(1., 1., 0.)),
			GroupSteerAgent,
			RunOnSpawn::default(),
			Separate::<GroupSteerAgent>::new(1.).scaled_dist(SCALE),
			Align::<GroupSteerAgent>::new(1.).scaled_dist(SCALE),
			Cohere::<GroupSteerAgent>::new(1.).scaled_dist(SCALE),
			Wander::new(1.).scaled_dist(SCALE),
		));
	}
}
