use beet_core::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::ItemFn;
use syn::LitInt;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;



struct TestParams {
	timeout_ms: Option<u64>,
}

impl Parse for TestParams {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let mut timeout_ms = None;

		while !input.is_empty() {
			let lookahead = input.lookahead1();
			if lookahead.peek(syn::Ident) {
				let ident: syn::Ident = input.parse()?;
				match ident.to_string().as_str() {
					"timeout_ms" => {
						input.parse::<Token![=]>()?;
						let lit: LitInt = input.parse()?;
						timeout_ms = Some(lit.base10_parse()?);
					}
					"tokio" => {
						// Skip tokio attribute, handled separately
					}
					_ => {
						return Err(syn::Error::new(
							ident.span(),
							format!("unknown test parameter: {}", ident),
						));
					}
				}

				// Parse optional comma
				if input.peek(Token![,]) {
					input.parse::<Token![,]>()?;
				}
			} else {
				return Err(lookahead.error());
			}
		}

		Ok(TestParams { timeout_ms })
	}
}

pub fn parse_sweet_test(
	attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
	let func = syn::parse::<ItemFn>(input)?;

	// Parse test parameters
	let test_params = if attr.is_empty() {
		TestParams { timeout_ms: None }
	} else {
		syn::parse::<TestParams>(attr.clone())?
	};

	let attrs = AttributeGroup::parse_punctated(attr.into())?;
	let is_tokio = attrs
		.iter()
		.any(|attr| attr.to_token_stream().to_string() == "tokio");

	// Generate params registration code if we have params
	let params_registration = if let Some(timeout_ms) = test_params.timeout_ms {
		quote! {
			sweet::register_test_params(
				sweet::TestCaseParams::new().with_timeout_ms(#timeout_ms)
			);
		}
	} else {
		quote! {}
	};

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
					#params_registration
					sweet::handle_async_test(async #block);
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
					#params_registration
					#block
				}
			}
		}
	}
	.xok()
}
