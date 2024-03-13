use proc_macro2::Span;
use proc_macro2::TokenStream;
use std::collections::HashMap;
use std::collections::HashSet;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Error;
use syn::Expr;
use syn::Ident;
use syn::Result;
use syn::Token;


#[derive(Default)]
pub struct AttributesMap {
	pub paths: HashMap<String, Span>,
	pub exprs: HashMap<String, Expr>,
}

impl AttributesMap {
	pub fn new(
		tokens: TokenStream,
		allowed_paths: &[&str],
		allowed_exprs: &[&str],
	) -> Result<Self> {
		let mut paths = HashMap::new();
		let mut exprs = HashMap::new();
		let all_allowed = allowed_exprs
			.iter()
			.chain(allowed_paths.iter())
			.map(|s| *s)
			.collect::<HashSet<&str>>();

		let items = Punctuated::<Expr, Token![,]>::parse_terminated
			.parse2(tokens)?
			.into_iter();

		for item in items {
			match item {
				Expr::Path(path) => {
					if let Some(ident) = path.path.get_ident() {
						check_allowed(ident, &all_allowed)?;
						paths.insert(ident.clone().to_string(), ident.span());
					} else {
						return Err(Error::new(
							path.span(),
							"Parse Error: Expected Attribute without assignment",
						));
					}
				}
				Expr::Assign(expr) => {
					match expr.left.as_ref() {
						Expr::Path(path) => {
							if let Some(ident) = path.path.get_ident() {
								check_allowed(ident, &all_allowed)?;
								exprs.insert(
									ident.clone().to_string(),
									expr.right.as_ref().clone(),
								);
							} else {
								return Err(Error::new(path.span(), 	"Parse Error: Expected Assignment, ie `foo=Bar"));
							}
						}
						other => {
							return Err(Error::new(
								other.span(),
								"Attribute Parser - Unexpected Error",
							))
						}
					}
				}
				other => {
					return Err(Error::new(
						other.span(),
						"Attribute Parser - Unexpected Error",
					))
				}
			}
		}
		Ok(Self { paths, exprs })
	}
}

fn check_allowed(ident: &Ident, allowed: &HashSet<&str>) -> Result<()> {
	let key = ident.to_string();
	if false == allowed.contains(key.as_str()) {
		let allowed_str = allowed
			.iter()
			.map(|s| s.to_string())
			.collect::<Vec<_>>()
			.join(", ");

		Err(Error::new(
			ident.span(),
			format!("Unknown Attribute: {key}\nExpected: {allowed_str}"),
		))
	} else {
		Ok(())
	}
}
