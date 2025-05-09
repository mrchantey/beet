use anyhow::Result;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::Block;
use syn::Expr;
use syn::Ident;
use syn::Lit;
use syn::LitStr;

/// Intermediate representation of an 'element' in an rsx tree.
/// Despite the web terminology, this is also used to represent
/// other types like Bevy entities.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ElementTokens {
	/// the name of the component, ie <MyComponent/>
	pub tag: Spanner<String>,
	/// fields of the component, ie <MyComponent foo=bar bazz/>
	pub attributes: Vec<RsxAttributeTokens>,
	/// special directives for use by both
	/// parser and WebNode pipelines, ie <MyComponent client:load/>
	pub meta: NodeMeta,
}

impl GetNodeMeta for ElementTokens {
	fn meta(&self) -> &NodeMeta { &self.meta }
	fn meta_mut(&mut self) -> &mut NodeMeta { &mut self.meta }
}

// used when a recoverable error is emitted
// impl Default for ElementTokens {
// 	fn default() -> Self { Self::fragment(Default::default()) }
// }

impl RustTokens for ElementTokens {
	fn into_rust_tokens(&self) -> TokenStream {
		let tag = self.tag.into_rust_tokens();
		let attributes =
			self.attributes.iter().map(|attr| attr.into_rust_tokens());
		let meta = &self.meta.into_rust_tokens();
		quote! {
			ElementTokens {
				tag: #tag,
				attributes: vec![#(#attributes),*],
				meta: #meta,
			}
		}
	}
}

/// Visit all [`ElementTokens`] in a tree, the nodes
/// should be visited before children, ie walked in DFS preorder.
pub trait ElementTokensVisitor<E = anyhow::Error> {
	fn walk_rsx_tokens(
		&mut self,
		mut visit: impl FnMut(&mut ElementTokens) -> Result<(), E>,
	) -> Result<(), E> {
		self.walk_rsx_tokens_inner(&mut visit)
	}
	fn walk_rsx_tokens_inner(
		&mut self,
		visit: &mut impl FnMut(&mut ElementTokens) -> Result<(), E>,
	) -> Result<(), E>;
}



#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RsxAttributeTokens {
	/// A block attribute, in jsx this is known as a spread attribute
	Block { block: Block, tracker: RustyTracker },
	/// A key attribute created by [`TokenStream`]
	Key { key: Spanner<String> },
	/// A key value attribute where the value is a literal like
	/// a string, number, or boolean
	KeyValueLit { key: Spanner<String>, value: Lit },
	/// A key value attribute where the value is a rust expression
	KeyValueExpr {
		key: Spanner<String>,
		value: Expr,
		tracker: RustyTracker,
	},
}
impl RsxAttributeTokens {
	// pub fn try_lit_str(expr: &Expr) -> Option<String> {
	// 	if let Expr::Lit(expr_lit) = expr {
	// 		if let syn::Lit::Str(lit_str) = &expr_lit.lit {
	// 			return Some(lit_str.value());
	// 		}
	// 	}
	// 	None
	// }
	pub fn lit_to_string(lit: &syn::Lit) -> String {
		match lit {
			syn::Lit::Int(lit_int) => lit_int.base10_digits().to_string(),
			syn::Lit::Float(lit_float) => lit_float.base10_digits().to_string(),
			syn::Lit::Bool(lit_bool) => lit_bool.value.to_string(),
			syn::Lit::Str(lit_str) => lit_str.value(),
			syn::Lit::ByteStr(lit_byte_str) => {
				String::from_utf8_lossy(&lit_byte_str.value()).into_owned()
			}
			syn::Lit::Byte(lit_byte) => lit_byte.value().to_string(),
			syn::Lit::Char(lit_char) => lit_char.value().to_string(),
			syn::Lit::Verbatim(lit_verbatim) => lit_verbatim.to_string(),
			syn::Lit::CStr(_) => unimplemented!(),
			_ => unimplemented!(),
		}
	}

	/// When testing for equality sometimes we dont want to compare spans and trackers.
	pub fn reset_spans_and_trackers(&mut self) {
		match self {
			RsxAttributeTokens::Block { tracker, .. } => {
				*tracker = RustyTracker::PLACEHOLDER
			}
			RsxAttributeTokens::Key { key, .. } => {
				key.tokens_span = proc_macro2::Span::call_site();
			}
			RsxAttributeTokens::KeyValueExpr { key, tracker, .. } => {
				key.tokens_span = proc_macro2::Span::call_site();
				*tracker = RustyTracker::PLACEHOLDER
			}
			RsxAttributeTokens::KeyValueLit { key, .. } => {
				key.tokens_span = proc_macro2::Span::call_site();
			}
		}
	}
}
impl RustTokens for RsxAttributeTokens {
	fn into_rust_tokens(&self) -> TokenStream {
		match self {
			RsxAttributeTokens::Block { block, tracker } => {
				let tracker = tracker.into_rust_tokens();
				quote! { RsxAttributeTokens::Block{
					block: #block,
					tracker: #tracker,
				} }
			}
			RsxAttributeTokens::Key { key } => {
				let key = key.into_rust_tokens();
				quote! { RsxAttributeTokens::Key{ key: #key } }
			}
			RsxAttributeTokens::KeyValueLit { key, value } => {
				let key = key.into_rust_tokens();
				quote! {
					RsxAttributeTokens::KeyValueLit {
						key: #key,
						value: #value,
					}
				}
			}
			RsxAttributeTokens::KeyValueExpr {
				key,
				value,
				tracker,
			} => {
				let key = key.into_rust_tokens();
				let tracker = tracker.into_rust_tokens();
				quote! {
					RsxAttributeTokens::KeyValueExpr {
						key: #key,
						value: #value,
						tracker: #tracker,
					}
				}
			}
		}
	}
}


/// A value that may have been created
#[derive(Debug, Clone)]
pub struct Spanner<T> {
	/// The span, if any, of the original value.
	/// This will be Span::call_site() if the value
	pub(crate) tokens_span: proc_macro2::Span,
	// file_span: FileSpan,
	value: T,
}


impl<T> Spanner<T> {
	pub fn new(value: T) -> Self {
		Self {
			value,
			tokens_span: proc_macro2::Span::call_site(),
		}
	}

	pub fn new_with_span(value: T, tokens_span: proc_macro2::Span) -> Self {
		Self { value, tokens_span }
	}
	pub fn tokens_span(&self) -> proc_macro2::Span { self.tokens_span }
	pub fn value(&self) -> &T { &self.value }
	pub fn into_value(self) -> T { self.value }
}

impl<T: std::hash::Hash> std::hash::Hash for Spanner<T> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.value.hash(state);
	}
}

impl<T: PartialEq> PartialEq for Spanner<T> {
	fn eq(&self, other: &Self) -> bool { self.value == other.value }
}
impl<T: Eq> Eq for Spanner<T> {}

impl<T: AsRef<str>> Spanner<T> {
	pub fn as_str(&self) -> &str { self.value.as_ref() }
	pub fn into_lit_str(&self) -> LitStr {
		LitStr::new(self.value.as_ref(), self.tokens_span)
	}
	pub fn into_ident(&self) -> Ident {
		Ident::new(self.value.as_ref(), self.tokens_span)
	}
}

impl Into<Spanner<String>> for LitStr {
	fn into(self) -> Spanner<String> {
		Spanner::new_with_span(self.value(), self.span())
	}
}
impl Into<Spanner<String>> for String {
	fn into(self) -> Spanner<String> { Spanner::new(self) }
}
impl<'a> Into<Spanner<String>> for &'a str {
	fn into(self) -> Spanner<String> { Spanner::new(self.to_string()) }
}
impl ToTokens for Spanner<String> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		self.into_lit_str().to_tokens(tokens);
	}
}

impl RustTokens for Spanner<String> {
	fn into_rust_tokens(&self) -> TokenStream {
		quote! { Spanner::new(#self.to_string()) }
	}
}

impl std::fmt::Display for Spanner<String> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.value)
	}
}
