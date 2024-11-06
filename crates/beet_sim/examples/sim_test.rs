use beet_sim::prelude::*;
use beetmash::prelude::*;
use bevy::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			BeetmashDefaultPlugins::with_native_asset_path("../../assets"),
			DefaultPlaceholderPlugin,
			BeetSimPlugin,
		))
		.add_systems(Startup, setup)
		.run();
}


fn setup(mut commands: Commands) {
	commands.spawn((Camera3d::default(), Transform::from_xyz(0., 0., 5.)));

	commands.spawn(Emoji::new("1F600"));
}
