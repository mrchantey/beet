use beet::examples::scenes;
use beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.add_systems(Startup, (
			scenes::camera_2d,
			scenes::space_scene,
			setup
		))
		.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	let planet = asset_server.load("spaceship_pack/planet_6.png");
	let ship = asset_server.load("spaceship_pack/ship_2.png");

	let target = commands
		.spawn((
			Name::new("Target"),
			FollowCursor2d,
			Transform::from_translation(Vec3::new(200., 0., 0.)),
			Sprite {
				image: planet,
				..default()
			},
		))
		.id();

	commands.spawn((
		Name::new("Agent"),
		Sprite {
			image: ship,
			..default()
		},
		RotateToVelocity2d,
		ForceBundle::default(),
		SteerBundle::default().scaled_dist(500.),
		SteerTarget::Entity(target),
		Seek::default(),
		TriggerDeferred::run(),
	));
}
