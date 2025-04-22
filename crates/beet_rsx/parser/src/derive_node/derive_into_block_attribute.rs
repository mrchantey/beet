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
	let fields = fields.iter().map(|field| {
		let ident = &field.ident;
		let ident_str = ident.to_string();
		let inner = if field.attributes.contains("flatten") {
			quote! {
				#ident.register_effects(loc)?;
			}
		} else if ident_str.starts_with("on") {
			runtime.register_event_tokens(&ident_str, ident)
		} else {
			// here we need to register effects for *all* fields, even static ones
			// because we dont know if they are reactive.
			let runtime_effect_registry = &runtime.effect;
			quote! {
				#runtime_effect_registry::register_attribute_effect(
					loc,
					#ident_str,
					#ident
				)
			}
		};
		// handle optional fields
		if field.is_optional() {
			quote! {
				if let Some(#ident) = self.#ident {
					#inner
				}
			}
		} else {
			quote! {{
				let #ident = self.#ident;
				#inner
			}}
		}
	});

	quote! {

		fn register_effects(self, loc: TreeLocation) -> beet::exports::anyhow::Result<()>{
			#(#fields)*
			Ok(())
		}
	}
}
