use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;


#[derive(Default)]
pub struct BertPlugin;

impl Plugin for BertPlugin {
	fn build(&self, app: &mut App) {

		app.add_plugins(ActionPlugin::<(
			SentenceFlow,
			SetSentenceOnUserInput,
			//we need OnInsert to derive reflect https://github.com/bevyengine/bevy/pull/14259
			// RunOnSentenceChange 
		)>::default())
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
