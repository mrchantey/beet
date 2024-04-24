use beet::prelude::*;
use bevy::prelude::*;

#[path = "misc/example_plugin.rs"]
mod example_plugin;
use example_plugin::ExamplePlugin;

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
				ForceBundle::default(),
				SteerBundle::default().scaled_to(500.),
				VelocityScalar(Vec3::new(1., 1., 0.)),
				GroupSteerAgent,
			))
			.with_children(|agent| {
				// behavior
				agent.spawn((Running, ParallelSelector)).with_children(
					|selector| {
						selector.spawn((
							Align::<GroupSteerAgent>::default(),
							RootIsTargetAgent,
						));
						selector.spawn((Wander::default(), RootIsTargetAgent));
					},
				);
			});
	}
}
