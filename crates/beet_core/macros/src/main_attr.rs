extern crate alloc;

use beet_core_shared::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;

pub fn impl_main_attr(
	attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
	if !attr.is_empty() {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"#[beet::main] does not accept arguments",
		));
	}

	let func = syn::parse::<ItemFn>(input)?;

	if func.sig.asyncness.is_none() {
		return Err(syn::Error::new_spanned(
			&func.sig.fn_token,
			"#[beet::main] requires an async function",
		));
	}

	if func.sig.ident != "main" {
		return Err(syn::Error::new_spanned(
			&func.sig.ident,
			"#[beet::main] can only be applied to the `main` function",
		));
	}

	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let block = &func.block;
	let attrs = &func.attrs;
	let vis = &func.vis;
	let output = &func.sig.output;

	Ok(quote! {
		#(#attrs)*
		#vis fn main() #output {
			#beet_core::prelude::async_ext::block_on_local_executor(async #block)
		}
	})
}
