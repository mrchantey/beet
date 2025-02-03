use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// Hash a span based on the start location
pub fn location_hash_tokens(span: &Span) -> TokenStream {
	let mut hash = DefaultHasher::new();
	span.start().hash(&mut hash);
	let hash = hash.finish();
	quote! {RustLocationHash::new(#hash)}
}


// pub fn effect_tokens()