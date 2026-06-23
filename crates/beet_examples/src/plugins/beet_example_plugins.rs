use beet_core::prelude::*;
use bevy::app::PluginGroupBuilder;

/// The canonical example plugin set, composed alongside [`BeetPlugins`](beet): it
/// adds the example capabilities, each inferred from a feature flag, and leaves the
/// runner and window to `BeetPlugins`. So `(BeetPlugins, BeetExamplePlugins)` is the
/// one combination every example uses, windowed or headless decided by the `winit`
/// flag on `BeetPlugins`, not by a separate example plugin.
#[derive(Default, Clone)]
pub struct BeetExamplePlugins;

impl PluginGroup for BeetExamplePlugins {
	fn build(self) -> PluginGroupBuilder {
		#[allow(unused_mut)]
		let mut builder = PluginGroupBuilder::start::<Self>();
		// the spatial/steering/animation scenes + their 2d/3d systems.
		#[cfg(feature = "bevy_default")]
		{
			builder = builder.add(crate::prelude::beet_example_plugin);
		}
		// the ml example plugins (bert + frozen-lake), which need the render scenes.
		#[cfg(all(feature = "bevy_default", feature = "ml"))]
		{
			builder = builder.add(crate::prelude::plugin_ml);
		}
		// the agent-thread runtime + chat UI + example tools, so a `<Thread>` `.bsx`
		// runs through the one binary (headless-friendly, no render needed).
		#[cfg(feature = "thread")]
		{
			builder = builder.add(crate::prelude::ThreadExamplesPlugin);
		}
		builder
	}
}
