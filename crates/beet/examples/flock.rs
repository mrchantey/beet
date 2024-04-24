use beet::prelude::*;
use bevy::prelude::*;

#[path = "misc/example_plugin.rs"]
mod example_plugin;
use example_plugin::ExamplePlugin;


#[derive(Component, Reflect)]
pub struct BoidMarker;




fn main() {
	let mut app = App::new();

	app /*-*/
		.add_plugins(ExamplePlugin)
		.add_systems(Startup, setup)
		.run()
	/*-*/;
}


fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	for _ in 0..10 {
		commands
			.spawn((
				SpriteBundle {
					texture: asset_server.load("spaceship_pack/ship_2.png"),
					..default()
				},
				RotateToVelocity2d,
				// Mass::default(),
				// Velocity::default(),
				// Impulse::default(),
				// Force::default(),
				ForceBundle::default(),
				SteerBundle::default().scaled_to(500.),
			))
			.with_children(|parent| {
				// behavior
				// parent.spawn((
				// 	Align::<BoidMarker>::default(),
				// 	Running,
				// 	TargetAgent(parent.parent_entity()),
				// ));
			});
	}
}
