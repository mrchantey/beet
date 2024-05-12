use beet::prelude::*;
use bevy::prelude::*;

#[path = "common/example_plugin_2d.rs"]
mod example_plugin_2d;
use example_plugin_2d::ExamplePlugin2d;
use example_plugin_2d::FollowCursor;


fn main() {
	let mut app = App::new();

	app /*-*/
		.add_plugins(ExamplePlugin2d)
		.add_systems(Startup, setup)
		.run()
	/*-*/;
}


fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	// target
	let target = commands
		.spawn((FollowCursor, SpriteBundle {
			transform: Transform::from_translation(Vec3::new(200., 0., 0.)),
			texture: asset_server.load("spaceship_pack/planet_6.png"),
			..default()
		}))
		.id();

	// agent
	commands
		.spawn((
			SpriteBundle {
				texture: asset_server.load("spaceship_pack/ship_2.png"),
				..default()
			},
			RotateToVelocity2d,
			ForceBundle::default(),
			SteerBundle::default().scaled_to(500.).with_target(target),
		))
		.with_children(|parent| {
			// behavior
			parent.spawn((Seek, Running, TargetAgent(parent.parent_entity())));
		});
}
