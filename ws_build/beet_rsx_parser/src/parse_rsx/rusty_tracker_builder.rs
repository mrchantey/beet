use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::quote;
use rapidhash::RapidHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// Used by [RstmlToRsx] *and* [RstmlToRsxTemplate] to assign a rusty tracker,
///  See [RustyTracker](beet_rsx::prelude::RustyTracker)
///  
/// # Footguns
/// - Users must be identical in their call ordering for indices to match
///	- Don't use TokenStream::to_string() and then hash the string.
/// 	the rsx! macro splits whitespace in the tokens
/// 	but parsing a syn::file doesn't (and it also inserts a wacky /n here and there).
///   Its possible to do a `.chars().filter(|c| !c.is_whitespace()).collect::<String>()`
///   but thats just extra work.
#[derive(Debug, Default)]
pub struct RustyTrackerBuilder {
	pub current_index: u32,
}

impl RustyTrackerBuilder {
	/// Provided stringified token stream,
	/// returns the tokens for the next next `RustyTracker` to build
	pub fn next_tracker(&mut self, hashable: impl Hash) -> TokenStream {
		let (index, tokens_hash) = self.next_index_hash(hashable);
		quote! {RustyTracker::new(#index, #tokens_hash)}
	}

	/// Provided stringified token stream,
	/// returns the tokens for the next next `RustyTracker` to build
	/// in ron format
	// #[deprecated = "these should be options on the builder"]
	pub fn next_tracker_ron(&mut self, hashable: impl Hash) -> TokenStream {
		let (index, tokens_hash) = self.next_index_hash(hashable);
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
	fn next_index_hash(
		&mut self,
		hashable: impl std::hash::Hash,
	) -> (u32, u64) {
		let mut hasher = RapidHasher::default_const();
		hashable.hash(&mut hasher);
		let tokens_hash = hasher.finish();

		let index = self.current_index;
		self.current_index += 1;
		// println!("index: {}, hash: {}", index, tokens_hash);
		(index, tokens_hash)
	}
}
