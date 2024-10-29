use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use emby::prelude::*;

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
		.run();
}

fn setup(mut commands: Commands) {
	commands.spawn((
		Camera3d::default(),
		RenderLayers::layer(0).with(RENDER_TEXTURE_LAYER),
		Transform::from_xyz(0., 1.6, 5.),
	));
}
