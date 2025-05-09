use beet_common::prelude::RustyTracker;
use std::hash::Hash;

/// Incremental builder for [`RustyTracker`]
#[derive(Debug, Default)]
pub struct RustyTrackerBuilder {
	pub current_index: u32,
}

impl RustyTrackerBuilder {
	/// Provided stringified token stream,
	/// returns the tokens for the next next `RustyTracker` to build
	pub fn next_tracker(&mut self, hashable: impl Hash) -> RustyTracker {
		let index = self.current_index;
		self.current_index += 1;

		RustyTracker::new_hashed(index, hashable)
	}
}
