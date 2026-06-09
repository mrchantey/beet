use crate::beet::prelude::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::asset::AssetMetaCheck;


/// A minimal app with flow and spatial
pub fn minimal_beet_example_plugin(app: &mut App) {
	app.add_plugins((
		MinimalPlugins,
		LogPlugin::default(),
		beet_example_plugin,
	));
}

/// A running app with flow and spatial
pub fn running_beet_example_plugin(app: &mut App) {
	app.add_plugins((
		// beet's `LogPlugin` replaces bevy's, so disable bevy's here and add ours.
		DefaultPlugins
			.set(AssetPlugin {
				// file_path: "../../assets".into(),
				meta_check: AssetMetaCheck::Never,
				..default()
			})
			.build()
			.disable::<bevy::log::LogPlugin>(),
		LogPlugin {
			level: Level::WARN,
			..default()
		},
		beet_example_plugin,
	))
	.add_systems(Update, (close_on_esc, toggle_fullscreen));
}

/// Simple default plugins
pub fn crate_test_beet_example_plugin(app: &mut App) {
	app.add_plugins((beet_example_plugin,));
}

pub fn beet_example_plugin(app: &mut App) {
	assert_local_assets();

	app.add_plugins((
		ActionPlugin,
		BeetSpatialPlugins,
		plugin_2d,
		plugin_3d,
		UiTerminalPlugin,
		// BeetDefaultPlugins,
		// DefaultReplicatePlugin,
	))
	.init_resource::<RandomSource>()
	.register_type::<Collectable>();
}

#[cfg(feature = "ml")]
pub fn plugin_ml(app: &mut App) {
	app.add_plugins((BeetMlPlugins, FrozenLakePlugin));
}

fn plugin_2d(app: &mut App) {
	app
		.add_systems(Update, follow_cursor_2d)
		// .add_systems(PreUpdate, auto_spawn::auto_spawn.before(PreTickSet))
		.add_systems(Update, randomize_position.in_set(PreTickSet))
		.add_systems(
			Update,
			(update_wrap_around, wrap_around)
			.chain()
			.in_set(PostTickSet),
		)
		.register_type::<RandomizePosition>()
		.register_type::<WrapAround>()
		.register_type::<FollowCursor2d>()
		/*_*/;
}

fn plugin_3d(app: &mut App) {
	app.add_systems(
			Update,(
			follow_cursor_3d,
			camera_distance,
			rotate_collectables,
			keyboard_controller,
		))
		.register_type::<FollowCursor3d>()
		.register_type::<KeyboardController>()
		.register_type::<CameraDistance>()
		/*-*/;
}
fn assert_local_assets() {
	#[cfg(target_arch = "wasm32")]
	return;
	#[allow(unreachable_code)]
	if !std::path::Path::new("assets/README.md").exists() {
		panic!(
			r#"
🌱🌱🌱

Welcome! This Beet example uses large assets that are stored remotely.
Until bevy supports http asset sources these must be downloaded manually:

Unix:

curl -o ./assets.tar.gz https://bevyhub-public.s3.us-west-2.amazonaws.com/assets.tar.gz
tar -xzvf ./assets.tar.gz
rm ./assets.tar.gz

Windows:

1. Download https://bevyhub-public.s3.us-west-2.amazonaws.com/assets.tar.gz
2. Unzip into `./assets`

🌱🌱🌱
"#
		);
	}
}

/// Toggles fullscreen mode when F11 is pressed.
fn toggle_fullscreen(
	input: When<Res<ButtonInput<KeyCode>>>,
	mut windows: Populated<&mut Window>,
) {
	use bevy::window::WindowMode;
	if input.just_pressed(KeyCode::F11) {
		for mut window in windows.iter_mut() {
			window.mode = match window.mode {
				WindowMode::Windowed => {
					WindowMode::BorderlessFullscreen(MonitorSelection::Current)
				}
				_ => WindowMode::Windowed,
			};
		}
	}
}
