use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::DeriveInput;
use syn::Result;

pub fn impl_into_block_attribute(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	// TODO this should be customizable via a derive macro attribute
	let runtime = RsxRuntime::default();

	let fields = PropsField::parse_all(&input)?;
	let fn_initial_attributes = fn_initial_attributes(&fields);
	let fn_register_effects = fn_register_effects(&runtime, &fields);

	let target_name = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	Ok(quote! {
		use beet::prelude::*;


		impl #impl_generics IntoBlockAttribute<Self> for #target_name #type_generics #where_clause {

			#fn_initial_attributes
			#fn_register_effects

		}
	})
}

fn fn_initial_attributes(fields: &[PropsField]) -> TokenStream {
	let fields = fields.iter().map(|field| {
		let ident = &field.ident;
		let ident_str = ident.to_string();
		let inner_ty_str = field.inner_ty.to_token_stream().to_string();
		let attr = match (ident_str.as_str(), inner_ty_str.as_str()) {
			// events are 'key only' attributes
			(key, _) if key.starts_with("on") => {
				quote! {(#ident_str.to_string(), None)}
			}
			// bools are 'key only' attributes
			(_, "bool") => {
				quote! {(#ident_str.to_string(), None)}
			}
			// all others must implement Into<String>
			// keep in mind the 'Optional' case refers to whether
			// the value is present, if so it must be Some<T>
			_ => {
				quote! {(#ident_str.into(), Some(#ident.to_string()))}
			}
		};

		if field.attributes.contains("flatten") {
			quote! {
				initial.extend(self.#ident.initial_attributes());
			}
		} else if field.is_optional() {
			quote! {
				#[allow(unused_variables)]
				if let Some(#ident) = &self.#ident {
					initial.push(#attr);
				}
			}
		} else {
			quote! {{
				#[allow(unused_variables)]
				let #ident = &self.#ident;
				initial.push(#attr);
			}}
		}
	});
	quote! {
		fn initial_attributes(&self) -> Vec<(String, Option<String>)>{
			let mut initial = Vec::default();
			#(#fields)*
			initial
		}
	}
}


fn fn_register_effects(
	runtime: &RsxRuntime,
	fields: &[PropsField],
) -> TokenStream {
	let fields = fields.iter().filter_map(|field| {
		let ident = &field.ident;
		let ident_str = ident.to_string();
		if field.attributes.contains("flatten") {
			Some(quote! {
				self.#ident.register_effects(loc)?;
			})
		} else if ident_str.starts_with("on") {
			let register_event =
				runtime.register_event_tokens(&ident_str, ident);
			if field.is_optional() {
				Some(quote! {
					if let Some(#ident) = self.#ident {
						#register_event
					}
				})
			} else {
				Some(quote! {{
					let #ident = self.#ident;
					#register_event
				}})
			}
		} else {
			// todo!("here we need to register BlockValue attributes same way as in tokens?");
			None
		}
	});

	quote! {

		fn register_effects(self, loc: TreeLocation) -> beet::exports::anyhow::Result<()>{
			#(#fields)*
			Ok(())
		}
	}
}
