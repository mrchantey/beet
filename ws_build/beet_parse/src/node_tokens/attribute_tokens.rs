use beet_common::prelude::*;
use bevy::prelude::*;
use syn::Block;
use syn::Expr;
use syn::Lit;

/// List of handles to the [`AttributeTokens`] that belong to this
/// entity. This may contain template directives until they are extracted.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Deref, DerefMut)]
pub struct AttributeTokensList(pub(crate) Vec<AttributeTokens>);


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeTokens {
	/// A block attribute, similar to a spread attribute in jsx
	/// ie `<div {vec![("hidden", true)]}>`
	Block {
		/// The attribute value, ie `{vec![("hidden", true)]}`
		value: Spanner<NonSendHandle<Block>>,
		tracker: RustyTracker,
	},
	/// A key attribute created by [`TokenStream`]
	/// ie `<div hidden>`
	Key {
		/// The attribute key, ie `hidden`
		key: Spanner<String>,
		/// used for generating rust tokens, this will only
		/// be `Some` if the node was generated from rust tokens.
		key_span: Option<NonSendHandle<proc_macro2::Span>>,
	},
	/// A key value attribute where the value is a literal like
	/// a string, number, or boolean
	/// ie `<div hidden=false>`
	KeyValueLit {
		/// The attribute key, ie `hidden`
		key: Spanner<String>,
		/// used for generating rust tokens, this will only
		/// be `Some` if the node was generated from rust tokens.
		key_span: Option<NonSendHandle<proc_macro2::Span>>,
		/// The attribute value, ie `false`
		value: Spanner<NonSendHandle<Lit>>,
	},
	/// A key value attribute where the value is a rust expression,
	/// ie `<div hidden={is_hidden}>`
	KeyValueExpr {
		/// The attribute key, ie `hidden`
		key: Spanner<String>,
		/// used for generating rust tokens, this will only
		/// be `Some` if the node was generated from rust tokens.
		key_span: Option<NonSendHandle<proc_macro2::Span>>,
		/// The attribute value, ie `{is_hidden}`
		value: Spanner<NonSendHandle<Expr>>,
		tracker: RustyTracker,
	},
}
