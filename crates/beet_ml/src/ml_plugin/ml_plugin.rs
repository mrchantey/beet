use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Default)]
pub struct MlPlugin {
	bert_config: BertConfig,
}


impl Plugin for MlPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ActionPlugin::<SentenceScorer>::default());

		app.insert_resource(Bert::new(self.bert_config.clone()).unwrap());

		let world = app.world_mut();
		world.init_component::<Sentence>();

		let mut registry =
			world.get_resource::<AppTypeRegistry>().unwrap().write();

		registry.register::<Sentence>();
	}
}
