//! Types for all parts of a node tree that are expressions.
//! All of these types contain the completely unparsed version of
//! their expression, to be be modified in the tokenization stage,
//! for example adding #[allow(unused_braces)] to block nodes
//! and appending `.into_bundle()`
use beet_core::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use send_wrapper::SendWrapper;
use syn::Expr;

/// An expression in some part of the tree
/// Almost all tokenizations must pass through this type
/// including:
/// - `rsx!` macro expressisons
/// - combinator expressions, first represented as [`CombinatorExpr`]
/// - template spawn funcs: `<MyTemplate/>`
///
/// Cases where this is not used:
/// - `#[derive(AttributeBlock)]`, tokenized directly with `.into_bundle`
///
/// The parsed output depends on the context in which this expression is used:
///
/// ## Node Blocks
///
/// Block Nodes that are expressions, any [`NodeExpr`] without an [`AttributeOf`]
/// is a block node.
///
/// ```ignore
/// rsx!{<div>{my_expr}</div>};
/// // templates also evaluate to blocks
/// rsx!{<MyTemplate/>};
/// ```
/// ## Attribute Blocks
///
/// any [`NodeExpr`] with an [`AttributeOf`] *without* an [`AttributeKey`]
/// is an attribute block.
/// This is known as the spread attribute in JSX, although rstml
/// doesn't require the `...` prefix.
/// ```ignore
/// rsx!{<span {props} />};
/// ```
/// ## Attribute Values
///
/// An expression that is used as the value of an attribute.
/// any [`NodeExpr`] with an [`AttributeOf`] *and* an [`AttributeKey`]
/// is an attribute value.
/// If the entity also has an [`TextNode`] this will be an [`Expr::Lit`].
/// ```ignore
/// rsx!{<span key={value} />};
/// ```
#[derive(Debug, Clone, Deref, Component, ToTokens)]
#[component(immutable)]
pub struct NodeExpr(pub SendWrapper<Expr>);

impl NodeExpr {
	/// Creates a new node expression from a syn expression.
	pub fn new(value: Expr) -> Self { Self(SendWrapper::new(value)) }
	/// Creates a new node expression from a block.
	pub fn new_block(value: syn::Block) -> Self {
		Self::new(syn::Expr::Block(syn::ExprBlock {
			block: value,
			attrs: Vec::new(),
			label: None,
		}))
	}
	/// Creates a new node expression from an identifier.
	pub fn new_ident(ident: syn::Ident) -> Self {
		Self::new(syn::Expr::Path(syn::ExprPath {
			attrs: Vec::new(),
			qself: None,
			path: ident.into(),
		}))
	}


	/// Returns a reference to the inner expression.
	pub fn borrow(&self) -> &syn::Expr { &*self.0 }
	/// Returns the inner expression with `#[allow(unused_braces)]` added to blocks.
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

	/// Generates tokens that convert this expression into a bundle.
	pub fn bundle_tokens(&self) -> TokenStream {
		let parsed = self.inner_parsed();
		quote! {#parsed.into_bundle()}
	}

	/// Called when this expression is in the position of a node or block attribute,
	/// ie `<div {my_expr} />`.
	pub fn insert_deferred(&self) -> TokenStream {
		let parsed = self.inner_parsed();
		quote! { OnSpawnDeferred::insert(#parsed.into_bundle()) }
	}

	/// Merges multiple expressions into a single deferred insert.
	pub fn merge_deferred(items: &Vec<Self>) -> TokenStream {
		let components = items.iter().map(|item| {
			let parsed = item.inner_parsed();
			quote! {
				#parsed.into_bundle()
			}
		});
		if components.len() == 1 {
			quote! {OnSpawnDeferred::insert(#(#components),*)}
		} else {
			quote! {OnSpawnDeferred::insert((#(#components),*))}
		}
	}
}
