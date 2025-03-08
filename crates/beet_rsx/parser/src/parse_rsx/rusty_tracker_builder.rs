use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use rapidhash::rapidhash;

/// Used by [RstmlToRsx] *and* [RstmlToRsxTemplate] to assign a rusty tracker,
///  
/// # Footgun
/// Both of these structs must be identical in their call ordering
///
///  See [RustyTracker](beet_rsx::prelude::RustyTracker)
///
#[derive(Debug, Default)]
pub struct RustyTrackerBuilder {
	pub current_index: u32,
}

impl RustyTrackerBuilder {
	pub fn next_index_hash(&mut self, tokens: TokenStream) -> (u32, u64) {
		// the rsx! macro splits whitespace in the tokens
		// but visiting tokens via runtime  file loading does not,
		// so we remove whitespace to ensure the hash is the same
		let tokens_hash =
			rapidhash(tokens.to_string().replace(" ", "").as_bytes());
		let index = self.current_index;
		self.current_index += 1;
		// println!("index: {}, hash: {}", index, tokens_hash);
		(index, tokens_hash)
	}


	/// tokens for the next next `RustyTracker` to build
	pub fn next_tracker(&mut self, val: impl ToTokens) -> TokenStream {
		let (index, tokens_hash) = self.next_index_hash(val.to_token_stream());
		quote! {RustyTracker::new(#index, #tokens_hash)}
	}

	/// [`Self::Next`] but outputs to ron syntax
	// #[deprecated = "these should be options on the builder"]
	pub fn next_tracker_ron(&mut self, val: impl ToTokens) -> TokenStream {
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
