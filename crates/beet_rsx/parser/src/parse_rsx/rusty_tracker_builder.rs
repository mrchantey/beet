use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// See [RustyTracker](beet_rsx::prelude::RustyTracker)
///
///
#[derive(Debug, Default)]
pub struct RustyTrackerBuilder {
	pub current_index: u32,
}

impl RustyTrackerBuilder {
	pub fn next_index_hash(&mut self, tokens: TokenStream) -> (u32, u64) {
		let mut hasher = DefaultHasher::new();
		tokens.to_string().hash(&mut hasher);
		let tokens_hash = hasher.finish();
		let index = self.current_index;
		self.current_index += 1;
		(index, tokens_hash)
	}


	/// tokens for the next next `RustyTracker` to build
	pub fn next(&mut self, val: impl ToTokens) -> TokenStream {
		let (index, tokens_hash) = self.next_index_hash(val.to_token_stream());
		quote! {RustyTracker::new(#index, #tokens_hash)}
	}

	/// convenience method for RstmlToRsx where we may not want to build trackers
	// #[deprecated = "these should be options on the builder"]
	pub fn next_optional(
		&mut self,
		val: impl ToTokens,
		build_trackers: bool,
	) -> TokenStream {
		if build_trackers {
			let tokens = self.next(val);
			quote! {Some(#tokens)}
		} else {
			quote! {None}
		}
	}

	/// [`Self::Next`] but outputs to ron syntax
	// #[deprecated = "these should be options on the builder"]
	pub fn next_ron(&mut self, val: impl ToTokens) -> TokenStream {
		let (index, tokens_hash) = self.next_index_hash(val.to_token_stream());
		let index = Literal::u32_unsuffixed(index);
		let tokens_hash = Literal::u64_unsuffixed(tokens_hash);

		quote! {
			RustyTracker(
				index: #index,
				tokens_hash: #tokens_hash
			)
		}
	}
}
