#![doc = include_str!("../README.md")]

beet_core::test_main!();

pub mod fetch_bytes;
#[cfg(feature = "bevy_default")]
pub mod frozen_lake;
pub mod language;
pub mod plugins;
#[cfg(feature = "bevy_default")]
pub mod rl;
#[cfg(feature = "bevy_default")]
pub mod rl_realtime;
#[cfg(test)]
pub mod test_utils;

/// Re-exports of the most commonly used types and functions in `beet_ml`.
///
/// The schedule sets ([`PreTickSet`], [`TickSet`], [`PostTickSet`]) are
/// intentionally **not** re-exported here — beet_spatial defines its own
/// identically-named sets and re-exporting from both via wildcard imports
/// creates an ambiguous-glob hard error. Refer to them by path:
/// `beet_ml::TickSet`.
pub mod prelude {
	pub use super::BeetMlPlugins;
	pub use crate::fetch_bytes::*;
	#[cfg(feature = "bevy_default")]
	pub use crate::frozen_lake::*;
	pub use crate::language::*;
	pub use crate::plugins::*;
	#[cfg(feature = "bevy_default")]
	pub use crate::rl::*;
	#[cfg(feature = "bevy_default")]
	pub use crate::rl_realtime::*;
	#[cfg(test)]
	pub use crate::test_utils::*;
}

use beet_core::prelude::*;
use bevy::app::PluginGroupBuilder;

/// Runs before [`TickSet`], for systems that prepare state for the tick
/// (ie spawning sessions or scoring inputs).
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PreTickSet;

/// Per-frame ml systems run in this set: environment stepping, policy
/// reads, sentence-similarity scoring.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TickSet;

/// Bookkeeping runs in this set, after [`TickSet`] (ie episode-end
/// handling, despawn-on-end).
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PostTickSet;

/// Plugins used for most beet ml apps.
#[derive(Default, Clone)]
pub struct BeetMlPlugins;

impl PluginGroup for BeetMlPlugins {
	fn build(self) -> PluginGroupBuilder {
		#[allow(unused_mut)]
		let mut builder = PluginGroupBuilder::start::<Self>()
			.add(ml_set_plugin)
			// shares Bevy's WGPU device with Burn when both render (`bevy_default`)
			// and the wgpu backend (`wgpu`) are present; a no-op otherwise, so it is
			// always safe to add. Must follow Bevy's render plugins in the final app
			// so its `finish` sees the RenderApp.
			.add(crate::prelude::SharedBurnWgpuPlugin);

		#[cfg(feature = "bevy_default")]
		(builder = builder
			.add(crate::prelude::language_plugin)
			.add(crate::prelude::RlPlugin));
		builder
	}
}

/// Orders [`PreTickSet`] → [`TickSet`] → [`PostTickSet`] in [`Update`].
fn ml_set_plugin(app: &mut App) {
	app.configure_sets(Update, TickSet.after(PreTickSet))
		.configure_sets(Update, PostTickSet.after(TickSet));
}
