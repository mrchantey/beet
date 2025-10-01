use beet_core::prelude::*;


/// A unique identifier for a stat,
/// when defined by [`StatMap::from_sim_descriptor`] this is the index in the [`SimDescriptor`].
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
pub struct StatId(pub usize);


impl Default for StatId {
	fn default() -> Self { Self(usize::MAX) }
}
