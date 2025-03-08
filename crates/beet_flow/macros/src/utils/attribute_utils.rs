use proc_macro2::TokenStream;
use syn::Expr;
use syn::Token;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

pub fn punctuated_args(tokens: TokenStream) -> syn::Result<Vec<Expr>> {
	let args = Punctuated::<Expr, Token![,]>::parse_terminated
		.parse2(tokens)?
		.into_iter()
		.collect::<Vec<_>>();
	Ok(args)
}
