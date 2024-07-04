use crate::prelude::*;
use beet::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

/// Kitchen sink plugin, this is all you need for
/// ### Rendering
/// - text
/// - 2d
/// - 3d
/// ### Beet
/// - steering
/// - machine learning
///
#[derive(Default)]
pub struct ExamplePlugins;

impl PluginGroup for ExamplePlugins {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add(ExampleReplicatePlugin)
			.add(ExampleMlPlugin)
	}
}


pub struct ExampleMlPlugin;

impl Plugin for ExampleMlPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			BertPlugin::default(),
			ActionPlugin::<InsertOnAssetEvent<RunResult, Bert>>::default(),
			AssetPlaceholderPlugin::<Bert>::default()
		));
	}
}
