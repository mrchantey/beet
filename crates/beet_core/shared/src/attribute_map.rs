use crate::*;
use proc_macro2::TokenStream;
use std::collections::HashMap;
use syn::Expr;
use syn::ExprAssign;
use syn::Result;
use syn::Token;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

pub struct AttributeMap(pub HashMap<String, Option<Expr>>);

impl std::ops::Deref for AttributeMap {
	type Target = HashMap<String, Option<Expr>>;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl AttributeMap {
	pub fn parse(tokens: TokenStream) -> Result<Self> {
		let args =
			Punctuated::<Expr, Token![,]>::parse_terminated.parse2(tokens)?;

		let mut map = HashMap::new();

		fn path_segment(path: syn::ExprPath) -> Result<String> {
			if path.path.segments.len() != 1 {
				synbail!(
					path,
					"Path must be a single segment, ie `foo` not `foo::bar`"
				);
			}
			Ok(path.path.segments[0].ident.to_string())
		}

		for arg in args {
			match arg {
				Expr::Assign(ExprAssign { left, right, .. }) => {
					let Expr::Path(expr_path) = *left else {
						synbail!(
							left,
							"Left hand side of assignment must be a path, ie `foo` in `foo = bar`"
						);
					};
					let left = path_segment(expr_path)?;
					map.insert(left, Some(*right));
				}
				Expr::Path(expr_path) => {
					let left = path_segment(expr_path)?;
					map.insert(left, None);
				}
				other => synbail!(
					other,
					"Only assignments and paths allowed, ie #[foo,bar = bazz]"
				),
			}
		}

		Ok(Self(map))
	}

	pub fn assert_types(
		&self,
		required: &[&str],
		optional: &[&str],
	) -> Result<&Self> {
		for key in required {
			if !self.0.contains_key(*key) {
				synbail!(key, "Missing required attribute `{}`", key);
			}
		}
		for key in self.0.keys() {
			if !optional.contains(&key.as_str())
				&& !required.contains(&key.as_str())
			{
				synbail!(
					key,
					"Invalid attribute key `{}`. Allowed keys are: {}",
					key,
					[optional, required].concat().join(", ")
				);
			}
		}
		Ok(self)
	}
}
