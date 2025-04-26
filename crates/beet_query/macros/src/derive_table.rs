use proc_macro2;
use proc_macro2::TokenStream;
use quote::quote;
use sweet::prelude::*;
use syn;
use syn::DeriveInput;
use syn::Result;

pub fn parse_derive_table(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> { 
	
	
	
	quote! {}.xok() 



}
