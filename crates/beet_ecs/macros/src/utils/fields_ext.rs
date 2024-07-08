use extend::ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Field;
use syn::Fields;

#[ext]
pub impl Fields {
	fn into_named(self) -> Vec<(TokenStream, Field)> {
		match self {
			Fields::Unit => Vec::new(),
			Fields::Named(fields) => fields
				.named
				.into_iter()
				.map(|f| {
					let ident = &f.ident.as_ref().unwrap();
					(quote! {#ident}, f)
				})
				.collect(),
			Fields::Unnamed(fields) => fields
				.unnamed
				.into_iter()
				.enumerate()
				.map(|(i, f)| {
					let ident = syn::Index::from(i);
					(quote! {#ident}, f)
				})
				.collect(),
		}
	}
}
