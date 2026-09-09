//! The field grammar shared by the `#[template]` and `#[action]` macros.
//!
//! Both lower a function parameter into a data-struct field, so the grammar
//! lives here once and cannot drift: templates spell it `#[prop]`, actions
//! spell it `#[field]`.
//!
//! - bare attribute -> optional, defaults by type
//! - `(default = expr)` -> optional, defaults to `expr`, forcing a manual
//!   `Default` impl
//! - an `Option<T>` parameter -> stored as `PropOpt<T>`, bound back to
//!   `Option<T>` in the body
//! - `(required)` -> stored as `PropOpt<T>`, validated by the consumer
//! - `(into)` -> call-site sugar, the body still binds the concrete type
//! - `(mut)` -> a mutable binding, only meaningful to `#[action]`
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
	/// type as written by the author — what the body binds
	pub ty: syn::Type,
	/// stored as `PropOpt<inner>` and validated by the consumer
	pub required: bool,
	/// the `T` of a declared `Option<T>` prop, stored as `PropOpt<T>`
	pub option_inner: Option<syn::Type>,
	/// `(default = expr)` default expression
	pub default_expr: Option<syn::Expr>,
	/// `(mut)`, binding the field mutably in the body
	pub mutable: bool,
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
			// `mut` is a keyword, so it can never parse as an `Expr`: lift it out
			// before the rest of the arguments reach [`AttributeMap`].
			let tokens = Self::take_mut(tokens, &mut mutable);
			let map = AttributeMap::parse(tokens)?;
			for key in map.keys() {
				match key {
					"required" => required = true,
					"default" => default_expr = map.get("default").cloned(),
					// `into` is call-site sugar; the body binds the concrete type.
					"into" => {}
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

		// a declared `Option<T>` prop stores `PropOpt<T>` and binds `Option<T>`.
		let option_inner =
			(!required).then(|| Self::option_inner_type(&ty)).flatten();

		Ok(Self {
			ident,
			ty,
			required,
			option_inner,
			default_expr,
			mutable,
			other_attrs,
		})
	}

	/// Whether the prop is stored as a `PropOpt<_>` (a required prop, or a
	/// declared `Option<T>` prop).
	pub fn is_opt(&self) -> bool {
		self.required || self.option_inner.is_some()
	}

	/// The field's stored type. A required prop or a declared `Option<T>` prop
	/// stores `PropOpt<inner>` (so the call-site conversion stays unambiguous);
	/// everything else stores its declared type.
	pub fn stored_ty(&self, beet_core: &syn::Path) -> TokenStream {
		if self.required {
			let ty = &self.ty;
			quote! { #beet_core::prelude::PropOpt<#ty> }
		} else if let Some(inner) = &self.option_inner {
			quote! { #beet_core::prelude::PropOpt<#inner> }
		} else {
			let ty = &self.ty;
			quote! { #ty }
		}
	}

	/// The struct field definition with forwarded attrs.
	///
	/// Fields are `pub` so a `<Name field=x/>` struct-literal patch resolves
	/// across module boundaries (the same crate, a different module).
	pub fn field_def(&self, beet_core: &syn::Path) -> TokenStream {
		let ident = &self.ident;
		let stored_ty = self.stored_ty(beet_core);
		let other_attrs = &self.other_attrs;
		quote! {
			#(#other_attrs)*
			pub #ident: #stored_ty
		}
	}

	/// The field's value inside a manual `Default` impl. Only a non-`PropOpt`
	/// field carries a `default_expr`; every other field defaults by type
	/// (`PropOpt` to `None`).
	pub fn default_value(&self) -> TokenStream {
		match (self.is_opt(), &self.default_expr) {
			(false, Some(expr)) => quote! { (#expr).into() },
			_ => quote! { ::core::default::Default::default() },
		}
	}

	/// The body binding that rebinds the stored field to the declared type:
	/// a `PropOpt`-stored optional prop unwraps to `Option<T>`. A required prop
	/// is unwrapped separately (after the missing check).
	pub fn body_binding(&self) -> Option<TokenStream> {
		if self.option_inner.is_some() && !self.required {
			let ident = &self.ident;
			Some(quote! { let #ident = #ident.into_inner(); })
		} else {
			None
		}
	}

	/// The `T` of an `Option<T>` type, or `None` if `ty` is not an `Option`.
	pub fn option_inner_type(ty: &syn::Type) -> Option<syn::Type> {
		let syn::Type::Path(tp) = ty else {
			return None;
		};
		let seg = tp.path.segments.last()?;
		if seg.ident != "Option" {
			return None;
		}
		let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
			return None;
		};
		args.args.iter().find_map(|arg| match arg {
			syn::GenericArgument::Type(inner) => Some(inner.clone()),
			_ => None,
		})
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

	/// Split a bare `mut` out of an attribute argument list, setting `mutable`
	/// and returning the remaining comma-separated arguments.
	fn take_mut(tokens: TokenStream, mutable: &mut bool) -> TokenStream {
		let mut kept: Vec<Vec<TokenTree>> = Vec::new();
		let mut current: Vec<TokenTree> = Vec::new();
		for tt in tokens {
			match &tt {
				TokenTree::Punct(punct) if punct.as_char() == ',' => {
					kept.push(core::mem::take(&mut current));
				}
				_ => current.push(tt),
			}
		}
		kept.push(current);
		kept.into_iter()
			.filter(|arg| {
				let is_mut = arg.len() == 1
					&& matches!(&arg[0], TokenTree::Ident(ident) if ident == "mut");
				*mutable |= is_mut;
				!is_mut && !arg.is_empty()
			})
			.map(|arg| arg.into_iter().collect::<TokenStream>())
			.collect::<Vec<_>>()
			.into_iter()
			.map(|arg| quote! { #arg, })
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
		assert!(prop.option_inner.is_none());
	}

	#[test]
	fn mut_and_default() {
		let prop =
			parse(syn::parse_quote! { #[field(mut, default = 3)] total: u32 });
		assert!(prop.mutable);
		assert!(prop.default_expr.is_some());
	}

	#[test]
	fn option_stores_prop_opt() {
		let prop = parse(syn::parse_quote! { #[field] name: Option<String> });
		assert!(prop.option_inner.is_some());
		assert!(prop.is_opt());
	}

	#[test]
	fn required_stores_prop_opt() {
		let prop = parse(syn::parse_quote! { #[field(required)] name: String });
		assert!(prop.required);
		assert!(prop.is_opt());
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
	fn other_attrs_kept() {
		let prop = parse(syn::parse_quote! {
			/// docs
			#[field]
			total: u32
		});
		assert_eq!(prop.other_attrs.len(), 1);
	}
}
