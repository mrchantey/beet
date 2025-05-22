use crate::prelude::*;
use bevy::prelude::*;
use syn::Expr;


/// An attribute key represented as tokens, usually either a string literal or a block.
///
/// ## Example
/// ```ignore
/// let key = "hidden";
/// rsx!{<span {key}=true />};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeKeyExpr(NonSendHandle<Expr>);
impl AttributeKeyExpr {
	pub fn new(value: NonSendHandle<Expr>) -> Self { Self(value) }
}


/// The tokens for an attribute value, usually a block or a literal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeValueExpr(NonSendHandle<Expr>);


impl AttributeValueExpr {
	pub fn new(value: NonSendHandle<Expr>) -> Self { Self(value) }
}


/// Tokens for an attribute without a specified key-value pair.
/// This is known as the spread attribute in JSX, although rust
/// apis dont require the `...` prefix.
/// ## Example
/// ```ignore
/// rsx!{<span {props} />};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeExpr(NonSendHandle<Expr>);


impl AttributeExpr {
	pub fn new(value: NonSendHandle<Expr>) -> Self { Self(value) }
}


