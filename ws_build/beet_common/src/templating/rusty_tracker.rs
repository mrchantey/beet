use rapidhash::RapidHasher;
use std::hash::Hasher;

/// A serializable counterpart to a [`RustyPart`]
/// This struct performs two roles:
/// 1. hydration splitting and joining
/// 2. storing the hash of a rusty part token stream, for hot reload diffing
///
/// The combination of an index and tokens hash guarantees the level of
/// diffing required to detect when a recompile is necessary.
/// ```rust ignore
/// let tree = rsx!{<div {rusty} key=73 key=rusty key={rusty}>other text{rusty}more text <Component key=value/></div>}
/// //							      ^^^^^             ^^^^^      ^^^^^             ^^^^^            ^^^^^^^^^^^^^^^^^^^
/// //							      attr blocks       idents     value blocks      node blocks      Component open tags
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RustyTracker {
	/// the order in which this part was visited by the syn::Visitor
	pub index: u32,
	/// a hash of the token stream for this part
	pub tokens_hash: u64,
}


impl RustyTracker {
	pub const PLACEHOLDER: Self = Self {
		index: u32::MAX,
		tokens_hash: u64::MAX,
	};

	pub fn new(index: u32, tokens_hash: u64) -> Self {
		Self { index, tokens_hash }
	}
	/// sometimes we want to diff a tree without the trackers
	pub fn clear(&mut self) {
		self.index = 0;
		self.tokens_hash = 0;
	}
	/// Used by [RustyTrackerBuilder] to assign a rusty tracker
	///  
	/// # Footguns
	/// - Users must be identical in their call ordering for indices to match
	///	- Don't use TokenStream::to_string() and then hash the string.
	/// 	the rsx! macro splits whitespace in the tokens
	/// 	but parsing a syn::file doesn't (and it also inserts a wacky /n here and there).
	///   Its possible to do a `.chars().filter(|c| !c.is_whitespace()).collect::<String>()`
	///   but thats just extra work.
	pub fn new_hashed(index: u32, hashable: impl std::hash::Hash) -> Self {
		let mut hasher = RapidHasher::default_const();
		hashable.hash(&mut hasher);
		let tokens_hash = hasher.finish();
		Self { index, tokens_hash }
	}
}

#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for RustyTracker {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let index = proc_macro2::Literal::u32_unsuffixed(self.index);
		let tokens_hash =
			proc_macro2::Literal::u64_unsuffixed(self.tokens_hash);
		quote::quote! { RustyTracker::new(#index, #tokens_hash) }
	}
}
