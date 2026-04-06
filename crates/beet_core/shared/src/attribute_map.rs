use crate::*;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use proc_macro2::TokenStream;
use syn::Expr;
use syn::ExprAssign;
use syn::Result;
use syn::Token;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

pub struct AttributeMap(Vec<(String, Option<Expr>)>);

impl AttributeMap {
	pub fn parse(tokens: TokenStream) -> Result<Self> {
		let args =
			Punctuated::<Expr, Token![,]>::parse_terminated.parse2(tokens)?;

		let mut map = Vec::new();

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
					map.push((left, Some(*right)));
				}
				Expr::Path(expr_path) => {
					let left = path_segment(expr_path)?;
					map.push((left, None));
				}
				other => synbail!(
					other,
					"Only assignments and paths allowed, ie #[foo,bar = bazz]"
				),
			}
		}

		Ok(Self(map))
	}

	pub fn get(&self, key: &str) -> Option<&Option<Expr>> {
		self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
	}

	pub fn contains_key(&self, key: &str) -> bool {
		self.0.iter().any(|(k, _)| k == key)
	}

	pub fn keys(&self) -> impl Iterator<Item = &str> {
		self.0.iter().map(|(k, _)| k.as_str())
	}

	pub fn insert(&mut self, key: String, value: Option<Expr>) {
		self.0.push((key, value));
	}

	pub fn assert_types(
		&self,
		required: &[&str],
		optional: &[&str],
	) -> Result<&Self> {
		for key in required {
			if !self.contains_key(key) {
				synbail!(key, "Missing required attribute `{}`", key);
			}
		}
		for key in self.keys() {
			if !optional.contains(&key) && !required.contains(&key) {
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
