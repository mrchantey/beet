use beet_core::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use quote::quote_spanned;
use rapidhash::RapidHashSet;
use send_wrapper::SendWrapper;
use std::sync::LazyLock;

// Collect elements to provide semantic highlight based on element tag.
// No differences between open tag and closed tag.
// Also multiple tags with same name can be present,
// because we need to mark each of them.
#[derive(Default, Deref, DerefMut, Component)]
pub struct CollectedElements(Vec<(String, SendWrapper<Span>)>);


impl CollectedElements {
	// TODO this is from the rstml example, havent yet looked into how to properly
	// implement it
	pub fn into_docs(&self) -> Result<Vec<TokenStream>> {
		// Mark some of elements as type,
		// and other as elements as fn in crate::docs,
		// to give an example how to link tag with docs.
		static ELEMENTS_AS_TYPE: LazyLock<RapidHashSet<&'static str>> =
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
