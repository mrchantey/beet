use proc_macro2::TokenStream;
use quote::quote;
use sweet::prelude::*;
use syn;
use syn::DeriveInput;

use syn::parse_macro_input;

pub fn impl_derive_to_tokens(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}



fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let name = &input.ident;
	let content = match &input.data {
		syn::Data::Struct(data_struct) => todo!(),
		syn::Data::Enum(data_enum) => todo!(),
		syn::Data::Union(data_union) => todo!(),
	};
	quote! {
		impl ::quote::ToTokens for #name {
			fn to_tokens(&self, tokens: &mut ::proc_macro2::TokenStream) {
				// #content
			}
		}
	}
	.xok()
}


#[cfg(test)]
mod test {
	// use crate::prelude::*;
	use super::parse;
	use quote::quote;
	use sweet::prelude::*;
	use syn::DeriveInput;

	#[test]
	fn works() {
		let input: DeriveInput = syn::parse_quote! {
			struct Test {
				inner: u32,
			}
		};
		input
			.xmap(parse)
			.unwrap()
			.xmap(|t| syn::parse2::<syn::ItemImpl>(t).unwrap())
			.xpect()
			.to_be(
				syn::parse2(quote! {
				impl::quote::ToTokens for Test {
							fn to_tokens(&self, tokens: &mut ::proc_macro2::TokenStream) {
								// TODO
							}
						}			})
				.unwrap(),
			);
	}
}
