use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;



pub fn parse_sweet_test(
	_attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
	let func = syn::parse::<ItemFn>(input)?;
	let out = if func.sig.asyncness.is_some() {
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
	} else {
		quote! {
			#[test]
			#func
		}
	};
	Ok(out)
}
