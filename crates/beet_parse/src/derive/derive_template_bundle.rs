use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Result;


/// For a struct where each field implements `IntoTemplateBundle`
pub fn impl_into_template_bundle(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let fields = NodeField::parse_derive_input(&input)?;
	let fields = fields.iter().map(|f| &f.ident);

	let target_name = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	Ok(quote! {
		use beet::prelude::*;

		impl #impl_generics IntoTemplateBundle<Self> for #target_name #type_generics #where_clause {
		fn into_node_bundle(self) -> impl Bundle{
			#[allow(unused_braces)]
			(#(self.#fields.into_node_bundle()),*)
			}
		}
	})
}
