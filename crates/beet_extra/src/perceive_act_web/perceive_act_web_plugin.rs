//! Wires the wasm browser head into the `beet` binary.
use super::*;
use beet_core::prelude::*;

/// Registers the browser head: the `<WebHead>` client-root template, its three
/// capability handlers and wire types, and the ECS -> `<img>` face observer, so a head
/// program `.bsx` the wasm binary runs resolves `<WebHead url="ws://.."/>`.
pub struct PerceiveActWebPlugin;

impl Plugin for PerceiveActWebPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<PerceiveActCorePlugin>()
			.register_type::<TakePhoto>()
			.register_type::<SpeakText>()
			.register_template::<WebHead>()
			// `ShowImage` is registered by `PerceiveActCorePlugin` (shared with the
			// native mock); this observer renders its recorded url into the `<img>`.
			.add_observer(render_face);
	}
}
