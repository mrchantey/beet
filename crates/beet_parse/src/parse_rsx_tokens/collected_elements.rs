use beet_core::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use quote::quote_spanned;
use send_wrapper::SendWrapper;
use std::sync::LazyLock;

/// Collects RSX element tags with their spans for semantic highlighting.
///
/// Tracks both open and closed tags, allowing multiple tags with the same name
/// to be present since each occurrence needs to be marked separately.
#[derive(Default, Deref, DerefMut, Component)]
pub struct CollectedElements(Vec<(String, SendWrapper<Span>)>);


impl CollectedElements {
	/// Converts collected elements into documentation tokens for semantic highlighting.
	pub fn into_docs(&self) -> Result<Vec<TokenStream>> {
		// Mark some of elements as type,
		// and other as elements as fn in crate::docs,
		// to give an example how to link tag with docs.
		static ELEMENTS_AS_TYPE: LazyLock<HashSet<&'static str>> =
			LazyLock::new(|| {
				vec!["html", "head", "meta", "link", "body"]
					.into_iter()
					.collect()
			});

		self.0
			.iter()
			.map(|(name, span)| {
				if ELEMENTS_AS_TYPE.contains(name.as_str()) {
					let element = quote_spanned!(**span => enum);
					quote!({#element X{}}).xok()
				} else {
					let element = quote_spanned!(**span => element);
					quote!(let _ = crate::docs::#element).xok()
				}
			})
			.collect()
	}
}
