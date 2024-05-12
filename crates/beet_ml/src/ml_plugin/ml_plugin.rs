use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;


#[derive(Default)]
pub struct MlPlugin;

impl Plugin for MlPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ActionPlugin::<SentenceScorer>::default())
			.init_asset::<Bert>()
			.init_asset_loader::<BertLoader>();

		#[cfg(feature = "beet_core")]
		app.add_plugins(
			ActionPlugin::<FindSentenceSteerTarget<With<Sentence>>>::default(),
		);


		let world = app.world_mut();
		world.init_component::<Sentence>();

		let mut registry =
			world.get_resource::<AppTypeRegistry>().unwrap().write();

		registry.register::<Sentence>();
	}
}
