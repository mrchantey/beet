use crate::beet::prelude::*;
use crate::prelude::*;
use bevy::prelude::*;

/// Some types, and ui elements
pub struct ExamplePluginBasics;

impl Plugin for ExamplePluginBasics {
	fn build(&self, app: &mut App) {
		app
    .add_plugins((
			ExampleDefaultPlugins,
			ExamplePluginTypesBasic,
		))
	/*-*/;
	}
}

/// All types and ui elements
pub struct ExamplePluginFull;

impl Plugin for ExamplePluginFull {
	fn build(&self, app: &mut App) {
		app.add_plugins((ExampleDefaultPlugins, ExamplePluginTypesFull));
	}
}


pub struct ExamplePluginTypesBasic;

impl Plugin for ExamplePluginTypesBasic {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			ExampleReplicatePlugin,
			DefaultBeetPlugins,
			BundlePlaceholderPlugin,
			UiTerminalPlugin,
			BeetDebugPluginBase,
			BeetDebugPluginStdout,
			Example2dPlugin,
			Example3dPlugin,
		))
		.register_type::<Collectable>();
	}
}
pub struct ExamplePluginTypesFull;

impl Plugin for ExamplePluginTypesFull {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			ExamplePluginTypesBasic,
			ExampleMlPlugin,
			FrozenLakePlugin,
		));
	}
}

pub struct ExampleMlPlugin;

impl Plugin for ExampleMlPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			// sentence selector
			BertPlugin::default(),
			ActionPlugin::<InsertOnAssetEvent<RunResult, Bert>>::default(),
			AssetPlaceholderPlugin::<Bert>::default(),
			ReadyOnAssetLoadPlugin::<Bert>::default(),
			// qtables (frozen lake)
			AssetPlaceholderPlugin::<QTable<GridPos, GridDirection>>::default(),
			ReadyOnAssetLoadPlugin::<QTable<GridPos, GridDirection>>::default(),
		))
		// fetch
		.add_plugins(ActionPlugin::<(
			InsertOnAssetEvent<RunResult, Bert>,
			InsertSentenceSteerTarget<Collectable>,
			RemoveOnTrigger<OnRunResult, Sentence>,
			RemoveOnTrigger<OnRun, SteerTarget>,
		)>::default())
			/*-*/;
	}
}



pub struct Example2dPlugin;

impl Plugin for Example2dPlugin {
	fn build(&self, app: &mut App) {
		app
		.add_plugins(ReadyOnAssetLoadPlugin::<Image>::default())
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
		.register_type::<RenderText>()
		.register_type::<WrapAround>()
		.register_type::<FollowCursor2d>()
		/*_*/;
	}
}

pub struct Example3dPlugin;

impl Plugin for Example3dPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			AnimationGraphPlaceholderPlugin,
			AssetPlaceholderPlugin::<AnimationClip>::default(),
			ReadyOnAssetLoadPlugin::<AnimationClip>::default(),
		))
		.add_systems(
			Update,
			(follow_cursor_3d, camera_distance, rotate_collectables),
		)
		.register_type::<FollowCursor3d>()
		.register_type::<CameraDistance>()
		/*-*/;
	}
}
