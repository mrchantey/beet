use beet_core::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::ItemFn;



pub fn parse_sweet_test(
	attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
	let func = syn::parse::<ItemFn>(input)?;
	let attrs = AttributeGroup::parse_punctated(attr.into())?;
	let is_tokio = attrs
		.iter()
		.any(|attr| attr.to_token_stream().to_string() == "tokio");


	let is_async = func.sig.asyncness.is_some();

	match (is_async, is_tokio) {
		(true, true) => {
			// wasm impl is recursive but oh well tokio dep is temp anyway
			quote! {
				#[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
				#[cfg_attr(target_arch = "wasm32", sweet::test)]
				#func
			}
		}
		(true, false) => {
			let ident = &func.sig.ident;
			let vis = &func.vis;
			let block = &func.block;
			let attrs = &func.attrs;
			// Check if #[should_panic] is present - it requires () return type
			quote! {
				#[test]
				#(#attrs)*
				#vis fn #ident() {
					sweet::prelude::register_async_test(async #block);
				}
			}
		}
		(false, _) => {
			quote! {
				#[test]
				#func
			}
		}
	}
	.xok()
}
