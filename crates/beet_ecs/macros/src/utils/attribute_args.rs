use proc_macro2::TokenStream;
use std::collections::HashMap;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Error;
use syn::Expr;
use syn::Result;
use syn::Token;

const ERR: &str = "Parse Error: Expected Assignment, ie `foo = \"bar\"`";

pub fn attributes_map(
	tokens: TokenStream,
	allowed: Option<&[&str]>,
) -> Result<HashMap<String, Option<Expr>>> {
	attributes_vec(tokens, allowed).map(|vec| vec.into_iter().collect())
}


pub fn attributes_vec(
	tokens: TokenStream,
	allowed: Option<&[&str]>,
) -> Result<Vec<(String, Option<Expr>)>> {
	let out = Punctuated::<Expr, Token![,]>::parse_terminated
		.parse2(tokens)?
		.iter()
		.map(|item| match item {
			Expr::Assign(expr) => match expr.left.as_ref() {
				Expr::Path(path) => {
					if let Some(ident) = path.path.get_ident() {
						Ok((
							ident.clone().to_string(),
							Some(expr.right.as_ref().clone()),
						))
					} else {
						Err(Error::new(path.span(), ERR))
					}
				}
				_ => Err(Error::new(expr.span(), ERR)),
			},
			Expr::Path(path) => {
				if let Some(ident) = path.path.get_ident() {
					Ok((ident.clone().to_string(), None))
				} else {
					Err(Error::new(path.span(), ERR))
				}
			}
			_ => Err(Error::new(item.span(), ERR)),
		})
		.collect::<Result<Vec<_>>>()?;

	if let Some(allowed) = allowed {
		for (key, _) in out.iter() {
			if false == allowed.contains(&key.to_string().as_str()) {
				let allowed =
					allowed.iter().map(|s| *s).collect::<Vec<_>>().join(", ");

				return Err(Error::new(
					key.span(),
					format!("Unknown Attribute: {key}\nExpected: {allowed}"),
				));
			}
		}
	}

	Ok(out)
}
