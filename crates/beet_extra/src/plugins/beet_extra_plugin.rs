#[allow(unused_imports)]
use beet_core::prelude::*;

/// The canonical example [`Plugin`], composed alongside [`BeetPlugins`](beet): it
/// adds the example capabilities, each inferred from a feature flag, and leaves the
/// runner and window to `BeetPlugins`. So `(BeetPlugins, BeetExtraPlugin)` is the one
/// combination every example uses, windowed or headless decided by the `winit` flag
/// on `BeetPlugins`, not by a separate example plugin. The facade adds this
/// automatically under its `extra` feature, so a `beet-cli` build only opts into the
/// sub-features.
///
/// A regular `Plugin` (not a `PluginGroup`) so the facade's `BeetPlugins` group can
/// nest it; idempotent inner plugins (`init_plugin`) keep a double-add safe.
#[derive(Default, Clone)]
pub struct BeetExtraPlugin;

impl Plugin for BeetExtraPlugin {
	#[allow(unused_variables)]
	fn build(&self, app: &mut App) {
		// the headless action examples (`examples/action/*.bsx`): example actions,
		// components and templates. Only beet_action/beet_core, so always added.
		app.add_plugins(crate::prelude::ActionExamplesPlugin);
		// the spatial/steering/animation scenes + their 2d/3d systems (and the render
		// `CharacterDrive` body for the agnostic `<Drive>` action).
		#[cfg(feature = "bevy_default")]
		app.add_plugins(crate::prelude::beet_extra_bevy_default_plugin);
		// the ml example plugins (bert + frozen-lake), which need the render scenes.
		#[cfg(all(feature = "bevy_default", feature = "ml"))]
		app.add_plugins(crate::prelude::beet_extra_ml_plugin);
		// the agent-thread runtime + chat UI + example tools, so a `<Thread>` `.bsx`
		// runs through the one binary (headless-friendly, no render needed), plus the
		// headless behaviour-tree example and the agent calculator toolset.
		#[cfg(feature = "thread")]
		app.add_plugins((
			crate::prelude::ThreadExamplesPlugin,
			crate::prelude::AgentExamplesPlugin,
		));
		// the cloudflare/aws deploy example types + templates, so an
		// `examples/infra/*.bsx` deployer runs through the one binary (headless).
		#[cfg(feature = "infra")]
		app.add_plugins(crate::prelude::InfraExamplesPlugin);
	}
}
