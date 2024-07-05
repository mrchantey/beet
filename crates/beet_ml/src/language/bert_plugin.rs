use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;


#[derive(Default)]
pub struct BertPlugin;

impl Plugin for BertPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ActionPlugin::<SentenceScorer>::default())
			.init_asset::<Bert>()
			.init_asset_loader::<BertLoader>()
			.register_type::<Sentence>()
			/*-*/;

		#[cfg(feature = "beet_core")]
		app.add_plugins(
			ActionPlugin::<FindSentenceSteerTarget<Sentence>>::default(),
		);

		let world = app.world_mut();
		world.init_component::<Sentence>();
	}
}
