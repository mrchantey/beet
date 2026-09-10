//! The field grammar shared by the `#[template]` and `#[action]` macros.
//!
//! Both lower a function parameter into a data-struct field, so the grammar
//! lives here once and cannot drift: templates spell it `#[prop]`, actions
//! spell it `#[field]`.
//!
//! - bare attribute -> optional, defaults by type
//! - `(default = expr)` -> optional, defaults to `expr`, forcing a manual
//!   `Default` impl
//! - an `Option<T>` parameter -> optional, defaults to `None`
//! - `(required)` -> stored as `Option<T>`, validated by the consumer, bound
//!   as `T` in the body
//! - `(mut)` -> a mutable binding, only meaningful to `#[action]`
//! - `(no_clone)` -> declared on the struct but not bound in the body, for a
//!   field too expensive to clone per call; only meaningful to `#[action]`
//! - a visibility (`pub`, `pub(crate)`, `pub(in path)`) -> narrows the field
//!   from the default `pub`
extern crate alloc;
use crate::prelude::*;
use alloc::vec::Vec;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;

/// A macro parameter lowered to a data-struct field.
#[derive(Debug)]
pub struct Prop {
	/// parameter name (also the field name)
	pub ident: syn::Ident,
	/// type as written by the author, what the body binds
	pub ty: syn::Type,
	/// stored as `Option<ty>` and validated by the consumer
	pub required: bool,
	/// `(default = expr)` default expression
	pub default_expr: Option<syn::Expr>,
	/// `(mut)`, binding the field mutably in the body
	pub mutable: bool,
	/// `(no_clone)`, declared on the struct but never bound in the body
	pub no_clone: bool,
	/// the declared visibility, defaulting to `pub` so a struct-literal patch
	/// resolves across module boundaries
	pub vis: syn::Visibility,
	/// non-grammar attributes (doc comments etc) kept on the field
	pub other_attrs: Vec<syn::Attribute>,
}

impl Prop {
	/// Whether a parameter carries the `attr_name` attribute, ie `#[prop]`.
	pub fn is_param(pt: &syn::PatType, attr_name: &str) -> bool {
		pt.attrs.iter().any(|attr| attr.path().is_ident(attr_name))
	}

	/// Parse one parameter into a [`Prop`], reading the `attr_name` attribute
	/// (`prop` for templates, `field` for actions).
	pub fn parse(pt: &syn::PatType, attr_name: &str) -> syn::Result<Self> {
		let ident = Self::param_ident(pt, attr_name)?;
		let ty = (*pt.ty).clone();

		let mut required = false;
		let mut mutable = false;
		let mut no_clone = false;
		let mut vis: syn::Visibility = syn::parse_quote!(pub);
		let mut default_expr = None;
		let mut other_attrs: Vec<syn::Attribute> = Vec::new();

		for attr in &pt.attrs {
			if !attr.path().is_ident(attr_name) {
				other_attrs.push(attr.clone());
				continue;
			}
			let tokens = match &attr.meta {
				syn::Meta::List(list) => list.tokens.clone(),
				syn::Meta::Path(_) => TokenStream::new(), // bare attribute
				syn::Meta::NameValue(_) => {
					synbail!(
						attr,
						"`#[{attr_name} = ..]` form is not supported, use #[{attr_name}(..)]"
					)
				}
			};
			// `mut` and a visibility are keyword forms that can never parse as an
			// `Expr`, so lift them out before the rest reaches [`AttributeMap`].
			let tokens = Self::take_keywords(tokens, &mut mutable, &mut vis)?;
			let map = AttributeMap::parse(tokens)?;
			for key in map.keys() {
				match key {
					"required" => required = true,
					"no_clone" => no_clone = true,
					"default" => default_expr = map.get("default").cloned(),
					"into" => synbail!(
						attr,
						"`#[{attr_name}(into)]` has been removed; a prop takes its own type, or an `Option` of it"
					),
					"all" => synbail!(
						attr,
						"`#[{attr_name}(all)]` has been removed; declare each field"
					),
					other => synbail!(
						attr,
						"unknown `#[{attr_name}({other})]` argument"
					),
				}
			}
		}

		Ok(Self {
			ident,
			ty,
			required,
			default_expr,
			mutable,
			no_clone,
			vis,
			other_attrs,
		})
	}

	/// The field's stored type: a required prop stores `Option<ty>` so a
	/// missing value is a graceful error, everything else stores its declared
	/// type.
	pub fn stored_ty(&self) -> TokenStream {
		let ty = &self.ty;
		match self.required {
			true => quote! { ::core::option::Option<#ty> },
			false => quote! { #ty },
		}
	}

	/// The struct field definition with forwarded attrs.
	///
	/// Fields default to `pub` so a `<Name field=x/>` struct-literal patch
	/// resolves across module boundaries: `rsx!` lowers a component tag to
	/// `Name { field: value.into_prop(), ..Default::default() }`, and both the
	/// named field and the functional update need to be nameable there. Declare
	/// a narrower visibility to opt out.
	pub fn field_def(&self) -> TokenStream {
		let ident = &self.ident;
		let vis = &self.vis;
		let stored_ty = self.stored_ty();
		let other_attrs = &self.other_attrs;
		quote! {
			#(#other_attrs)*
			#vis #ident: #stored_ty
		}
	}

	/// The field's value inside a manual `Default` impl: the `default_expr`
	/// where one was declared (converted with `Into`, so an `Option<T>` prop
	/// takes a bare `T`), else the stored type's own default.
	pub fn default_value(&self) -> TokenStream {
		match (self.required, &self.default_expr) {
			(false, Some(expr)) => quote! { (#expr).into() },
			_ => quote! { ::core::default::Default::default() },
		}
	}

	/// Extract the identifier from a simple parameter pattern.
	pub fn param_ident(
		pt: &syn::PatType,
		macro_name: &str,
	) -> syn::Result<syn::Ident> {
		Ok(Self::param_pat_ident(pt, macro_name)?.ident)
	}

	/// Extract the full binding pattern (preserving `mut`) from a simple
	/// parameter.
	pub fn param_pat_ident(
		pt: &syn::PatType,
		macro_name: &str,
	) -> syn::Result<syn::PatIdent> {
		match pt.pat.as_ref() {
			syn::Pat::Ident(pi) => Ok(pi.clone()),
			other => {
				synbail!(
					other,
					"`#[{macro_name}]` parameters must be plain identifiers"
				)
			}
		}
	}

	/// Coerce a typed function argument, rejecting `self`.
	pub fn typed_arg<'a>(
		arg: &'a syn::FnArg,
		macro_name: &str,
	) -> syn::Result<&'a syn::PatType> {
		match arg {
			syn::FnArg::Typed(pt) => Ok(pt),
			syn::FnArg::Receiver(recv) => {
				synbail!(recv, "`#[{macro_name}]` functions cannot take `self`")
			}
		}
	}

	/// Split the keyword-shaped arguments (`mut`, a visibility) out of an
	/// attribute argument list, returning the remainder for [`AttributeMap`].
	///
	/// Neither form can reach `AttributeMap`: both open with a keyword, which
	/// never parses as an `Expr`.
	fn take_keywords(
		tokens: TokenStream,
		mutable: &mut bool,
		vis: &mut syn::Visibility,
	) -> syn::Result<TokenStream> {
		let mut kept: Vec<TokenStream> = Vec::new();
		for arg in Self::split_args(tokens) {
			let first = arg.clone().into_iter().next();
			match &first {
				Some(TokenTree::Ident(ident)) if ident == "mut" => {
					*mutable = true;
				}
				Some(TokenTree::Ident(ident)) if ident == "pub" => {
					*vis = syn::parse2(arg)?;
				}
				_ => kept.push(arg),
			}
		}
		Ok(kept.into_iter().map(|arg| quote! { #arg, }).collect())
	}

	/// Split a comma-separated attribute argument list into its non-empty
	/// top-level arguments.
	fn split_args(tokens: TokenStream) -> Vec<TokenStream> {
		let mut args: Vec<Vec<TokenTree>> = Vec::new();
		let mut current: Vec<TokenTree> = Vec::new();
		for tt in tokens {
			match &tt {
				TokenTree::Punct(punct) if punct.as_char() == ',' => {
					args.push(core::mem::take(&mut current));
				}
				_ => current.push(tt),
			}
		}
		args.push(current);
		args.into_iter()
			.filter(|arg| !arg.is_empty())
			.map(|arg| arg.into_iter().collect())
			.collect()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use alloc::string::ToString;

	fn pat_type(arg: syn::FnArg) -> syn::PatType {
		match arg {
			syn::FnArg::Typed(pt) => pt,
			_ => panic!("expected a typed argument"),
		}
	}

	fn parse(arg: syn::FnArg) -> Prop {
		Prop::parse(&pat_type(arg), "field").unwrap()
	}

	#[test]
	fn bare_attr() {
		let prop = parse(syn::parse_quote! { #[field] total: u32 });
		assert!(!prop.required);
		assert!(!prop.mutable);
		assert_eq!(prop.stored_ty().to_string(), "u32");
	}

	#[test]
	fn mut_and_default() {
		let prop =
			parse(syn::parse_quote! { #[field(mut, default = 3)] total: u32 });
		assert!(prop.mutable);
		assert!(prop.default_expr.is_some());
	}

	#[test]
	fn option_stores_as_declared() {
		let prop = parse(syn::parse_quote! { #[field] name: Option<String> });
		assert_eq!(prop.stored_ty().to_string(), "Option < String >");
	}

	#[test]
	fn required_stores_option() {
		let prop = parse(syn::parse_quote! { #[field(required)] name: String });
		assert!(prop.required);
		assert_eq!(
			prop.stored_ty().to_string(),
			":: core :: option :: Option < String >"
		);
	}

	#[test]
	fn into_is_rejected() {
		Prop::parse(
			&pat_type(syn::parse_quote! { #[field(into)] name: String }),
			"field",
		)
		.unwrap_err()
		.to_string()
		.contains("removed")
		.then_some(())
		.unwrap();
	}

	#[test]
	fn unknown_key_errors() {
		let err = Prop::parse(
			&pat_type(syn::parse_quote! { #[field(bogus)] name: String }),
			"field",
		)
		.unwrap_err()
		.to_string();
		assert!(err.contains("unknown"));
	}

	#[test]
	fn visibility_narrows_the_field() {
		let prop = parse(syn::parse_quote! { #[field(pub(crate))] total: u32 });
		assert!(matches!(prop.vis, syn::Visibility::Restricted(_)));
		let prop = parse(syn::parse_quote! { #[field] total: u32 });
		assert!(matches!(prop.vis, syn::Visibility::Public(_)));
	}

	#[test]
	fn no_clone_is_unbound() {
		let prop = parse(
			syn::parse_quote! { #[field(no_clone, pub(super))] big: Vec<u8> },
		);
		assert!(prop.no_clone);
		assert!(matches!(prop.vis, syn::Visibility::Restricted(_)));
	}

	#[test]
	fn other_attrs_kept() {
		let prop = parse(syn::parse_quote! {
			/// docs
			#[field]
			total: u32
		});
		assert_eq!(prop.other_attrs.len(), 1);
	}
}
