use crate::prelude::Deref;
#[cfg(feature = "bevy")]
use bevy::prelude::*;
use std::num::ParseIntError;
use std::str::FromStr;

/// A value guaranteed to be unique for every [`RsxNode`] *instance* in an
/// application before interactivity. This is what allows for reconciliation
/// in hydration and template reloading.
///
/// This technique is also the reason there can only be a single entrypoint for
/// a document, app etc, the tree idx is incremented as items are rendered.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deref,
)]
#[cfg_attr(
	feature = "bevy",
	derive(bevy::prelude::Component, bevy::prelude::Reflect)
)]
#[cfg_attr(feature = "bevy", reflect(Default, Component))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeIdx(u32);

impl TreeIdx {
	pub fn new(idx: u32) -> Self { Self(idx) }
}

/// An id incrementer for mappers, similar to the [TreeLocation] visitor pattern.
/// This pattern only works if implemented consistently between mappers.
/// The #1 rule is that [`Self::next`] must be called for *every single* [`RsxNode`].
/// Even if you don't use the value, it must still be visited to keep
/// the rsx id consistency.
/// - [`RsxNode::Fragment`]
/// - [`RsxNode::Block`]
/// - [`RsxBlock::initial`]
/// - [`RsxComponent::root`]
#[derive(Debug, Default)]
pub struct TreeIdxIncr(u32);

impl TreeIdxIncr {
	/// Call this before visiting any node.
	pub fn next(&mut self) -> TreeIdx {
		let idx = self.0;
		self.0 += 1;
		TreeIdx(idx)
	}
}


impl std::fmt::Display for TreeIdx {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
impl FromStr for TreeIdx {
	type Err = ParseIntError;
	fn from_str(s: &str) -> Result<Self, Self::Err> { s.parse().map(Self) }
}

impl Into<TreeIdx> for u32 {
	fn into(self) -> TreeIdx { TreeIdx::new(self) }
}
