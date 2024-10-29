use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;
use emby::prelude::*;
// use bevy::render::view::RenderLayers;


fn main() {
	App::new()
		.add_plugins((crate_test_beet_example_plugin, plugin_ml, EmbyPlugin))
		.insert_resource(BeetDebugConfig::default())
		.add_systems(
			Startup,
			(
				setup,
				beetmash::core::scenes::lighting_3d,
				beetmash::core::scenes::ground_3d,
				beetmash::core::scenes::ui_terminal_input,
				emby::scenes::spawn_barbarian,
			),
		)
		// .add_systems(Update,disable_barbarian)
		.run();
}


fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands.spawn((
		Camera3d::default(),
		Transform::from_xyz(0., 1.6, 5.), // .looking_at(Vec3::ZERO, Vec3::Y),
	));

	commands.insert_resource(EmojiMap::new(&asset_server));
}
