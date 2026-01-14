use crate::shared_utils::AttributeGroup;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;

pub fn parse_test_attr(
	attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
	let func = syn::parse::<ItemFn>(input)?;

	// Convert proc_macro::TokenStream to proc_macro2::TokenStream
	let attr_tokens: TokenStream = attr.into();

	// Parse attributes using AttributeGroup
	let attrs = if attr_tokens.is_empty() {
		AttributeGroup { attributes: vec![] }
	} else {
		// Create a synthetic attribute to parse
		let synthetic_attr: syn::Attribute =
			syn::parse_quote!(#[beet(#attr_tokens)]);
		AttributeGroup::parse(&[synthetic_attr], "beet")?
	};

	attrs.validate_allowed_keys(&["timeout_ms", "tokio"])?;

	let timeout_ms = attrs.get_value_parsed::<syn::LitInt>("timeout_ms")?;
	let is_tokio = attrs.contains("tokio");

	// Build test params
	let params_expr = if let Some(timeout_lit) = timeout_ms {
		quote! {
			beet_core::testing::TestCaseParams::new().with_timeout_ms(#timeout_lit)
		}
	} else {
		quote! {
			beet_core::testing::TestCaseParams::new()
		}
	};

	let is_async = func.sig.asyncness.is_some();

	Ok(match (is_async, is_tokio) {
		(true, true) => {
			// wasm impl is recursive but oh well tokio dep is temp anyway
			quote! {
				#[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
				#[cfg_attr(target_arch = "wasm32", beet_core::sweet_test)]
				#func
			}
		}
		(true, false) => {
			let ident = &func.sig.ident;
			let vis = &func.vis;
			let block = &func.block;
			let attrs = &func.attrs;
			quote! {
				#[test]
				#(#attrs)*
				#vis fn #ident() {
					beet_core::testing::register_test(
						#params_expr,
						async #block
					);
				}
			}
		}
		(false, _) => {
			let ident = &func.sig.ident;
			let vis = &func.vis;
			let block = &func.block;
			let attrs = &func.attrs;
			let sig_inputs = &func.sig.inputs;
			let sig_output = &func.sig.output;

			quote! {
				#[test]
				#(#attrs)*
				#vis fn #ident(#sig_inputs) #sig_output {
					#block
				}
			}
		}
	})
}
