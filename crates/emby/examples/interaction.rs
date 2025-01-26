use beet_examples::prelude::*;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use emby::prelude::*;

fn main() {
	App::new()
		.add_plugins((crate_test_beet_example_plugin, plugin_ml, EmbyPlugin))
		.add_systems(
			Startup,
			(
				setup,
				bevyhub::core::scenes::lighting_3d,
				bevyhub::core::scenes::ground_3d,
				bevyhub::core::scenes::ui_terminal_input,
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
