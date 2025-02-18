use crate::prelude::*;

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
pub struct RsxIdxIncr(RsxIdx);

impl RsxIdxIncr {
	/// Call this before visiting any node.
	pub fn next(&mut self) -> RsxIdx {
		let idx = self.0;
		self.0 += 1;
		idx
	}
}
