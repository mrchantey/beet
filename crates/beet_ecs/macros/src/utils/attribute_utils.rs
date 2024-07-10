use proc_macro2::TokenStream;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::Expr;
use syn::Token;

pub fn punctuated_args(tokens: TokenStream) -> syn::Result<Vec<Expr>> {
	let args = Punctuated::<Expr, Token![,]>::parse_terminated
		.parse2(tokens)?
		.into_iter()
		.collect::<Vec<_>>();
	Ok(args)
}


// #[deprecated]
// pub fn build_generic_funcs(
// 	funcs: &Vec<Expr>,
// 	generic_funcs: &Vec<Expr>,
// 	type_generics: &TypeGenerics,
// ) -> Vec<TokenStream> {
// 	let mut all_funcs = generic_funcs
// 		.iter()
// 		.map(|ident| {
// 			quote! { #ident::#type_generics }
// 		})
// 		.collect::<Vec<_>>();
// 	all_funcs.extend(funcs.iter().map(|ident| {
// 		quote! { #ident }
// 	}));
// 	all_funcs
// }
