use anyhow::Result;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::Block;
use syn::Expr;
use syn::Ident;
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
	Block { block: Block },
	/// A key attribute created by [`TokenStream`]
	Key { key: Spanner<String> },
	/// A key value attribute created by [`TokenStream`]
	KeyValue { key: Spanner<String>, value: Expr },
}
impl RsxAttributeTokens {
	pub fn try_lit_str(expr: &Expr) -> Option<String> {
		if let Expr::Lit(expr_lit) = expr {
			if let syn::Lit::Str(lit_str) = &expr_lit.lit {
				return Some(lit_str.value());
			}
		}
		None
	}
}
impl RustTokens for RsxAttributeTokens {
	fn into_rust_tokens(&self) -> TokenStream {
		match self {
			RsxAttributeTokens::Block { block } => {
				quote! { RsxAttributeTokens::Block{ block: #block} }
			}
			RsxAttributeTokens::Key { key } => {
				let key = key.into_rust_tokens();
				quote! { RsxAttributeTokens::Key{ key: #key } }
			}
			RsxAttributeTokens::KeyValue { key, value } => {
				let key = key.into_rust_tokens();
				quote! {
					RsxAttributeTokens::KeyValue {
						key: #key,
						value: #value,
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
	tokens_span: proc_macro2::Span,
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
