//! Wires the wasm browser head into the `beet` binary.
use super::*;
use beet_core::prelude::*;

/// Registers the browser head: the `<WebHead>` client-root template, its three
/// capability handlers and wire types, and the ECS -> `<img>` face observer, so a head
/// program `.bsx` the wasm binary runs resolves `<WebHead url="ws://.."/>`.
pub struct PerceiveActWebPlugin;

impl Plugin for PerceiveActWebPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<ClientRole>()
			.register_type::<WhoAmI>()
			.register_type::<TakePhoto>()
			.register_type::<SpeakText>()
			.register_type::<SetEmotion>()
			.register_type::<Emotion>()
			.register_template::<WebHead>()
			.add_observer(render_face);
	}
}
