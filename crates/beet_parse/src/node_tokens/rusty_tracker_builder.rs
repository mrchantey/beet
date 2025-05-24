use beet_common::prelude::RustyTracker;
use beet_rsx_combinator::prelude::ToHtml;
use beet_rsx_combinator::types::RsxAttributes;
use beet_rsx_combinator::types::RsxElementName;
use quote::ToTokens;
use rapidhash::RapidHasher;
use rstml::atoms::OpenTag;
use std::hash::Hash;
use std::hash::Hasher;

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

	/// hash the tag and attributes for [`beet_rsx_combinator`] [`RsxElementName`] and [`RsxAttributes`],
	pub fn next_rsx_el(
		&mut self,
		element_name: &RsxElementName,
		attributes: &RsxAttributes,
	) -> RustyTracker {
		let index = self.current_index;
		self.current_index += 1;
		let mut hasher = RapidHasher::default_const();

		element_name.to_html().hash(&mut hasher);

		for attr in attributes.0.iter() {
			attr.to_html().hash(&mut hasher);
		}

		let tokens_hash = hasher.finish();

		RustyTracker::new(index, tokens_hash)
	}

	/// hash the tag and attributes of an [`rstml`] [`OpenTag`], ignoring its spans.
	pub fn next_rstml_el(&mut self, open_tag: &OpenTag) -> RustyTracker {
		let index = self.current_index;
		self.current_index += 1;
		let mut hasher = RapidHasher::default_const();

		open_tag.name.to_string().hash(&mut hasher);

		// at this stage directives are still attributes, which
		// is good because we want to hash those too
		for attr in open_tag.attributes.iter() {
			attr.to_token_stream().to_string().hash(&mut hasher);
		}

		let tokens_hash = hasher.finish();

		RustyTracker::new(index, tokens_hash)
	}
}
