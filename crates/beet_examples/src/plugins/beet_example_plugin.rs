use crate::beet::prelude::*;
use crate::prelude::*;
use beetmash::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;


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
		BeetmashDefaultPlugins::with_beetmash_assets(),
		beet_example_plugin,
	));
}

/// Simple default plugins
pub fn crate_test_beet_example_plugin(app: &mut App) {
	app.add_plugins((
		BeetmashDefaultPlugins {
			default_asset_path: "../../assets".into(),
			..default()
		},
		beet_example_plugin,
	));
}

pub fn beet_example_plugin(app: &mut App) {
	app.add_plugins((
		// BeetmashDefaultPlugins::with_beetmash_assets(),
		DefaultPlaceholderPlugin,
		UiTerminalPlugin,
		BeetDefaultPlugins,
		BeetDebugPlugin,
		DefaultReplicatePlugin,
		temp_patches,
	))
	.add_plugins((plugin_spatial, plugin_2d, plugin_3d))
	.register_type::<Collectable>();
}


/// For apps and scenes that use beet_spatial
fn plugin_spatial(app: &mut App) {
	app
		.add_plugins(ActionPlugin::<(
			RemoveOnTrigger<OnRunResult, SteerTarget>,
			RemoveOnTrigger<OnRunResult, Velocity>,
			InsertOnTrigger<OnRun, Velocity>,
			RemoveOnTrigger<OnRun, Velocity>,
		)>::default())
		/*-*/;
}


pub fn plugin_ml(app: &mut App) {
	app.add_plugins((
		FrozenLakePlugin,
		// sentence selector
		BertPlugin::default(),
		AssetPlaceholderPlugin::<Bert>::default(),
		ReadyOnAssetLoadPlugin::<Bert>::default(),
		// qtables (frozen lake)
		AssetPlaceholderPlugin::<QTable<GridPos, GridDirection>>::default(),
		ReadyOnAssetLoadPlugin::<QTable<GridPos, GridDirection>>::default(),
	))
	// fetch
	.add_plugins(ActionPlugin::<(
		InsertSentenceSteerTarget<Collectable>,
		RemoveOnTrigger<OnRunResult, Sentence>,
	)>::default())
		/*-*/;
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
			.run_if(|res: Option<Res<WrapAround>>| res.is_some())
			.in_set(PostTickSet),
		)
		.register_type::<AutoSpawn>()
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
			ik_spawner.never_param_warn(),
			update_emoji_swapper.never_param_warn()
		))
		.register_type::<IkSpawner>()
		.register_type::<FollowCursor3d>()
		.register_type::<KeyboardController>()
		.register_type::<CameraDistance>()
		/*-*/;
}
