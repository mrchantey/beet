use super::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;



/// The parser for the #[sweet_test] attribute
pub struct SweetTestAttr;

impl SweetTestAttr {
	pub fn parse(
		_attr: proc_macro::TokenStream,
		input: proc_macro::TokenStream,
	) -> syn::Result<TokenStream> {
		let func = syn::parse::<ItemFn>(input)?;


		if let Some(non_async) = non_async(&func) {
			return Ok(non_async);
		}

		let func_native = parse_native(&func)?;
		let func_wasm = parse_wasm(&func)?;

		let out = quote! {
			#func_native
			#func_wasm
		};

		Ok(out)
	}
}


/// non async tests are just #[test]
fn non_async(func: &ItemFn) -> Option<TokenStream> {
	if func.sig.asyncness.is_some() {
		return None;
	}
	let out = quote! {
		#[test]
		#func
	}
	.into();
	Some(out)
}
