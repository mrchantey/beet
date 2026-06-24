mod bert;
pub use self::bert::*;
mod trigger_with_user_sentence;
pub use self::trigger_with_user_sentence::*;

use beet_core::prelude::*;
use std::borrow::Cow;

/// Registers [`Sentence`] and the [`Bert`] asset (+ `.ron` loader) for
/// embedding-driven actions like [`NearestSentence`] and (with the
/// `spatial` feature) [`SentenceSteerTarget`]. Also wires the
/// [`trigger_with_user_sentence`] observer so [`TriggerWithUserSentence`]
/// entities respond to [`UserMessage`].
pub fn language_plugin(app: &mut App) {
	app.register_type::<Sentence>()
		.init_asset::<Bert>()
		.init_asset_loader::<BertLoader>()
		.add_observer(trigger_with_user_sentence);
	app.world_mut().register_component::<Sentence>();
}

/// A natural-language label attached to either an agent (the prompt) or a
/// child behavior (a candidate). Used by sentence-similarity selectors.
#[derive(Debug, Clone, Component, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct Sentence(pub Cow<'static, str>);

impl Sentence {
	/// Create a [`Sentence`] from any string-like type.
	pub fn new(s: impl Into<Cow<'static, str>>) -> Self { Self(s.into()) }
}

impl Default for Sentence {
	fn default() -> Self { Self::new("placeholder") }
}
