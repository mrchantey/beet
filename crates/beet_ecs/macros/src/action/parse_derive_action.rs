use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::DeriveInput;
use syn::Expr;
use syn::Result;
use syn::Token;

pub fn parse_derive_action(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> Result<TokenStream> {
	let item = syn::parse::<DeriveInput>(item)?;
	// let args = syn::parse::<Args>
	// let args = Attribute::parse_inner(input).parse(attr)?;
	let omits = get_omits(attr.into())?;

	let exprs = vec![
		parse_quote!(Debug),
		parse_quote!(Default),
		parse_quote!(Clone),
		parse_quote!(Component),
		parse_quote!(Reflect),
		parse_quote!(Action),
	]
	.into_iter()
	.filter(|expr| !omits.contains(expr))
	.collect::<Punctuated<Expr, Token![,]>>();

	Ok(quote! {
	use ::beet::prelude::*;
	use ::beet::exports::*;
	#[derive(#exprs)]
	#[reflect(Default, Component)]
		#item
	})
}

fn get_omits(attrs: TokenStream) -> Result<Vec<Expr>> {
	let val = Punctuated::<Expr, Token![,]>::parse_terminated
		.parse2(attrs)?
		.into_iter()
		.collect::<Vec<Expr>>();
	Ok(val)
}


// fn get_omits(attrs:&Vec<Attribute>)->Result<Vec<Expr>>{
// 	for item in attrs.iter() {
// 		if item.path().is_ident("omit") {
// 			return Ok(Punctuated::<Expr, Token![,]>::parse_terminated
// 				.parse2(item.meta.to_token_stream())?
// 				.into_iter()
// 				.collect::<Vec<Expr>>())
// 		}
// 	}
// 	Ok(Default::default())
// }
