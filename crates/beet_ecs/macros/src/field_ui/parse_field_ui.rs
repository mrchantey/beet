use crate::*;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::DeriveInput;
use syn::Result;


pub fn parse_field_ui(item: proc_macro::TokenStream) -> Result<TokenStream> {
	let input = syn::parse::<DeriveInput>(item)?;
	let ident = &input.ident;
	let attrs = input.attrs;
	// let attrs = vec![];
	let variant =
		parse_type_attrs(&ident.to_token_stream(), &attrs, &quote! {reflect})?;
	// println!("VARIANT: \n{:?}", variant);

	let out: TokenStream = match input.data {
		syn::Data::Struct(input) => parse_struct(input)?,
		syn::Data::Enum(input) => parse_enum(input, variant)?,
		syn::Data::Union(_) => unimplemented!(),
	};


	Ok(quote! {
		use beet::prelude::*;
		impl IntoFieldUi for #ident{
			fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
				#out
			}
		}
	})
}
