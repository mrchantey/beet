use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;


/// Native async tests are currently just `#[tokio::test]`
pub fn parse_native(func: &ItemFn) -> syn::Result<TokenStream> {
	let out = quote! {
		#[cfg(not(target_arch = "wasm32"))]
		#[tokio::test]
		#func
	};
	Ok(out)
}
