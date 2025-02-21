mod bert;
mod run_with_user_sentence;
pub use self::bert::*;
pub use self::run_with_user_sentence::*;
mod bert_config;
pub use self::bert_config::*;
mod bert_loader;
pub use self::bert_loader::*;
mod nearest_sentence;
pub use self::nearest_sentence::*;
mod sentence_embeddings;
pub use self::sentence_embeddings::*;


use beet_flow::prelude::*;
use bevy::prelude::*;

#[derive(Default)]
pub struct LanguagePlugin;

impl Plugin for LanguagePlugin {
	fn build(&self, app: &mut App) {
		app.init_asset::<Bert>()
			.init_asset_loader::<BertLoader>()
			.register_type::<Sentence>()
			.add_observer(run_with_user_sentence::<()>);

		let world = app.world_mut();
		world.register_component::<Sentence>();
	}
}
