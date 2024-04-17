use crate::prelude::*;
use bevy::prelude::*;

#[derive(Default)]
pub struct MlPlugin {
	bert_config: BertConfig,
}


impl Plugin for MlPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(Bert::new(self.bert_config.clone()).unwrap());
	}
}
