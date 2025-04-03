use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::DeriveInput;
use syn::Result;

pub fn impl_into_rsx_attributes(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let fields = PropsField::parse_all(&input)?;

	let push_fields = fields
		.iter()
		.map(|field| {
			let ident = &field.inner.ident;
			let ident_str = ident.to_token_stream().to_string();

			if field.attributes.contains("flatten") {
				quote! {
					attributes.extend(Into::<Vec<RsxAttribute>>::into(self.#ident));
				}
			} else if field.is_optional() {
				quote! {
					if let Some(#ident) = self.#ident {
						attributes.push(RsxAttribute::KeyValue{
							key: #ident_str.into(),
							value: #ident.into()
						});
					}
				}
			} else {
				quote! {
					attributes.push(RsxAttribute::KeyValue{
						key: #ident_str.into(),
						value: self.#ident.into()
					});
				}
			}
		})
		.collect::<Vec<_>>();

	let target_name = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();


	Ok(quote! {
		use beet::prelude::*;


		impl #impl_generics Into<Vec<RsxAttribute>> for #target_name #type_generics #where_clause {
			fn into(self) -> Vec<RsxAttribute> {
				let mut attributes:Vec<RsxAttribute> = Default::default();
				#(#push_fields)*
				attributes
			}
		}
	})
}
