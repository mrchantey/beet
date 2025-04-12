use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use sweet::prelude::*;
use syn::DeriveInput;
use syn::Result;

pub fn impl_into_rsx_attributes(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let fields = PropsField::parse_all(&input)?;
	let into_initial = impl_into_initial(&fields)?;
	let register_effects = impl_register_effects(&fields)?;

	let target_name = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();


	Ok(quote! {
		use beet::prelude::*;


		impl #impl_generics IntoBlockAttribute<Self> for #target_name #type_generics #where_clause {
			fn into_initial_attributes(self) -> Vec<RsxAttribute>{
				let mut attributes:Vec<RsxAttribute> = Default::default();
				#(#into_initial)*
				attributes
			}

			fn register_effects(self, _loc: TreeLocation) -> beet::exports::anyhow::Result<()>{
				#(#register_effects)*
				todo!()
			}

		}
	})
}

fn impl_into_initial(fields: &[PropsField]) -> Result<Vec<TokenStream>> {
	fields
		.iter()
		// this is just for initial values, events are only registered
		.filter(|field| !field.ident.to_string().starts_with("on"))
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
					attributes.extend(self.#ident.into_initial_attributes());
				}
			} else if field.is_optional() {
				quote! {
					#[allow(unused_variables)]
					if let Some(#ident) = &self.#ident {
						attributes.push(#attr);
					}
				}
			} else {
				quote! {{
					#[allow(unused_variables)]
					let #ident = &self.#ident;
					attributes.push(#attr);
				}}
			}
		})
		.collect::<Vec<_>>()
		.xok()
}


fn impl_register_effects(fields: &[PropsField]) -> Result<Vec<TokenStream>> {
	fields
		.iter()
		.filter(|field| field.ident.to_string().starts_with("on"))
		.map(|field| {
			// let ident = &field.ident;
			// let ident_str = ident.to_token_stream().to_string();
			// let attr =
			// 	match field.unwrapped.to_token_stream().to_string().as_ref() {
			// 		"bool" => quote! { RsxAttribute::Key{
			// 			key: #ident_str.into()
			// 		} },
			// 		_ => {
			// 			quote! {RsxAttribute::KeyValue{
			// 				key: #ident_str.into(),
			// 				value: #ident.into()
			// 			}}
			// 		}
			// 	};
			// if field.attributes.contains("flatten") {
			// 	quote! {
			// 		self.#ident.register_effects(loc);
			// 	}
			// } else if field.is_optional() {
			// 	quote! {
			// 		if let Some(#ident) = &self.#ident {
			// 			self.#ident.register_effects(loc);
			// 		}
			// 	}
			// } else {
			// 	quote! {{
			// 		let #ident = &self.#ident;
			// 		self.#ident.register_effects(loc);
			// 	}}
			// }
			Default::default()
		})
		.collect::<Vec<_>>()
		.xok()
}
