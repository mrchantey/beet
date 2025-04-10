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
			let ident = &field.ident;
			let ident_str = ident.to_token_stream().to_string();


			let attr =
				match field.unwrapped.to_token_stream().to_string().as_ref() {
					"bool" => quote! { RsxAttribute::Key{
						key: #ident_str.into()
					} },
					_ => {
						quote! {RsxAttribute::KeyValue{
							key: #ident_str.into(),
							value: #ident.into()
						}}
					}
				};


			if field.attributes.contains("flatten") {
				quote! {
					attributes.extend(Into::<Vec<RsxAttribute>>::into(self.#ident));
				}
			} else if field.is_optional() {
				quote! {
					#[allow(unused_variables)]
					if let Some(#ident) = self.#ident {
						attributes.push(#attr);
					}
				}
			} else {
				quote! {{
					#[allow(unused_variables)]
					let #ident = self.#ident;
					attributes.push(#attr);
				}}
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
