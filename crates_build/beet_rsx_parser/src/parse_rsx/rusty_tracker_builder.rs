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
	/// Provided stringified token stream,
	/// returns the tokens for the next next `RustyTracker` to build
	pub fn next_tracker(&mut self, val: impl ToTokens) -> TokenStream {
		let (index, tokens_hash) = self.next_index_hash(val);
		quote! {RustyTracker::new(#index, #tokens_hash)}
	}

	/// Provided stringified token stream,
	/// returns the tokens for the next next `RustyTracker` to build
	/// in ron format
	// #[deprecated = "these should be options on the builder"]
	pub fn next_tracker_ron(&mut self, val: impl ToTokens) -> TokenStream {
		let (index, tokens_hash) = self.next_index_hash(val);
		let index = Literal::u32_unsuffixed(index);
		let tokens_hash = Literal::u64_unsuffixed(tokens_hash);

		quote! {
			RustyTracker(
				index: #index,
				tokens_hash: #tokens_hash
			)
		}
	}

	/// provided a stringified token stream, returns the next index and hash
	fn next_index_hash(&mut self, tokens: impl ToTokens) -> (u32, u64) {
		// the rsx! macro splits whitespace in the tokens
		// but parsing a syn::file doesn't (and it also inserts a wacky /n here and there)
		// so we remove whitespace to ensure the hash is the same
		let tokens_hash = rapidhash(
			tokens
				.to_token_stream()
				.to_string()
				.chars()
				.filter(|c| !c.is_whitespace())
				.collect::<String>()
				.as_bytes(),
		);
		let index = self.current_index;
		self.current_index += 1;
		// println!("index: {}, hash: {}", index, tokens_hash);
		(index, tokens_hash)
	}
}
