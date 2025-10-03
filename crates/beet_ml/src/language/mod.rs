#[cfg(feature = "candle")]
mod bert;
#[cfg(feature = "candle")]
pub use self::bert::*;
mod trigger_with_user_sentence;
pub use self::trigger_with_user_sentence::*;

use beet_core::prelude::*;
use beet_flow::prelude::*;
use std::borrow::Cow;

#[derive(Default)]
pub struct LanguagePlugin;

impl Plugin for LanguagePlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Sentence>()
			.add_observer(trigger_with_user_sentence::<RunPayload>);

		#[cfg(feature = "candle")]
		app.init_asset::<Bert>().init_asset_loader::<BertLoader>();

		let world = app.world_mut();
		world.register_component::<Sentence>();
	}
}


/// This component is for use with [`SentenceFlow`]. Add to either the agent or a child behavior.
#[derive(Debug, Clone, Component, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Sentence(pub Cow<'static, str>);
impl Sentence {
	pub fn new(s: impl Into<Cow<'static, str>>) -> Self { Self(s.into()) }
}

impl Default for Sentence {
	fn default() -> Self { Self::new("placeholder") }
}
