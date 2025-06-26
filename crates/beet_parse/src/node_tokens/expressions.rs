//! Types for all parts of a node tree that are expressions.
//! All of these types contain the completely unparsed version of
//! their expression, to be be modified in the tokenization stage,
//! for example adding #[allow(unused_braces)] to block nodes
//! and appending `.into_node_bundle()`
use beet_common::as_beet::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use send_wrapper::SendWrapper;
use syn::Expr;

/// Tokens for an attribute without a specified key-value pair.
/// This is known as the spread attribute in JSX, although rust
/// parsers usually dont require the `...` prefix.
/// ## Example
/// ```ignore
/// rsx!{<span {props} />};
/// rsx!{<span key={value} />};
/// ```
#[derive(Debug, Clone, Deref, Component, ToTokens)]
#[component(immutable)]
pub struct AttributeExpr(pub SendWrapper<Expr>);


impl AttributeExpr {
	pub fn new(value: Expr) -> Self { Self(SendWrapper::new(value)) }
	pub fn new_block(value: syn::Block) -> Self {
		Self::new(syn::Expr::Block(syn::ExprBlock {
			block: value,
			attrs: Vec::new(),
			label: None,
		}))
	}
	pub fn borrow(&self) -> &syn::Expr { &*self.0 }
	/// ensure blocks have `#[allow(unused_braces)]`
	pub fn inner_parsed(&self) -> Expr {
		match self.borrow().clone() {
			syn::Expr::Block(mut block) => {
				block.attrs.push(syn::parse_quote! {
					#[allow(unused_braces)]
				});
				Expr::Block(block)
			}
			expr => expr,
		}
	}

	/// Called when this expression is in the position of a block attribute,
	/// ie `<div {my_expr} />`.
	pub fn node_bundle_tokens(&self) -> TokenStream {
		let parsed = self.inner_parsed();
		quote! { #parsed.into_node_bundle() }
	}
	/// Called when this expression is in the position of a block attribute,
	/// ie `<div key={my_expr} />`.
	pub fn attribute_bundle_tokens(&self) -> TokenStream {
		let parsed = self.inner_parsed();
		quote! { #parsed.into_attribute_bundle() }
	}
}



/// Nodes that are expressions, including block nodes and templates.
/// ## Example
/// ```ignore
/// rsx!{<div>{my_expr}</div>};
/// ```
#[derive(Debug, Clone, Deref, Component, ToTokens)]
#[component(immutable)]
pub struct NodeExpr(pub send_wrapper::SendWrapper<syn::Expr>);


impl NodeExpr {
	pub fn new(value: syn::Expr) -> Self {
		Self(send_wrapper::SendWrapper::new(value))
	}
	pub fn new_block(value: syn::Block) -> Self {
		Self::new(syn::Expr::Block(syn::ExprBlock {
			block: value,
			attrs: Vec::new(),
			label: None,
		}))
	}
	pub fn take(self) -> syn::Expr { self.0.take() }
	pub fn borrow(&self) -> &syn::Expr { &*self.0 }

	/// Create the tokens for this expression to be instantiated
	pub fn node_bundle_tokens(&self) -> TokenStream {
		match self.borrow() {
			syn::Expr::Block(block) => {
				quote! {
					#[allow(unused_braces)]
					#block.into_node_bundle()
				}
			}
			expr => {
				quote! { #expr.into_node_bundle() }
			}
		}
	}
}




/// The partially parsed equivalent of a [`RsxParsedExpression`](beet_rsx_combinator::types::RsxParsedExpression).
///
/// [`beet_rsx_combinator`] is very different from macro/tokens based parsers.
/// A fundamental concept is support for mixed expressions `let foo = <div/>;`
/// which means we need to parse `let foo =` seperately from `<div/>`. So the
/// element is added in a similar way to [`rstml`] so that we can still
/// apply scoped styles etc, but the hierarchy is not exactly correct, as
/// elements are parsed in the order they are defined not applied.
/// It can later be combined into a single expression
/// `let foo = (NodeTag("div"),ElementNode{self_closing=true});`
///
#[derive(Default, Component, Deref, DerefMut, ToTokens)]
pub struct CombinatorExpr(pub Vec<CombinatorExprPartial>);

/// A section of a [`CombinatorExpr`],
/// a 1:1 mapping from [`RsxTokensOrElement`](beet_rsx_combinator::types::RsxTokensOrElement)
#[derive(ToTokens)]
pub enum CombinatorExprPartial {
	/// partial expressions must be a string as it may not be a valid
	/// TokenTree at this stage, for instance {let foo = <bar/>} will be split into
	/// `{let foo =` + `<bar/>` + `}`, unclosed braces are not a valid [`TokenStream`]
	Tokens(String),
	/// Reference to the entity containing the [`NodeTag`], [`ElementNode`] etc
	Element(Entity),
}
