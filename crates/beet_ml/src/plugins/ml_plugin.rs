use crate::*;
use bevy::prelude::*;

pub struct MyPlugin;

impl Plugin for MyPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
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
}
