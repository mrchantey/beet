use crate::as_beet::*;
use bevy::prelude::*;
use send_wrapper::SendWrapper;
use syn::Expr;

/// Tokens for an attribute without a specified key-value pair.
/// This is known as the spread attribute in JSX, although rust
/// parsers usually dont require the `...` prefix.
/// ## Example
/// ```ignore
/// rsx!{<span {props} />};
/// ```
#[derive(Debug, Clone, Deref, Component, ToTokens)]
#[component(immutable)]
pub struct AttributeExpr(pub SendWrapper<Expr>);


impl AttributeExpr {
	pub fn new(value: Expr) -> Self { Self(SendWrapper::new(value)) }
	pub fn take(self) -> Expr { self.0.take() }
}


/// An attribute key represented as tokens, usually either a string literal or a block.
///
/// ## Example
/// ```ignore
/// let key = "hidden";
/// rsx!{<span {key}=true />};
/// ```
#[derive(Debug, Clone, Deref, Component, ToTokens)]
#[component(immutable)]
pub struct AttributeKeyExpr(pub SendWrapper<Expr>);
impl AttributeKeyExpr {
	pub fn new(value: Expr) -> Self { Self(SendWrapper::new(value)) }
	pub fn inner(&self) -> &Expr { &*self.0 }
	pub fn take(self) -> Expr { self.0.take() }
}


/// The tokens for an attribute value, usually a block or a literal.
#[derive(Debug, Clone, Deref, Component, ToTokens)]
#[component(immutable)]
pub struct AttributeValueExpr(pub SendWrapper<Expr>);


impl AttributeValueExpr {
	pub fn new(value: Expr) -> Self { Self(SendWrapper::new(value)) }
	pub fn inner(&self) -> &Expr { &*self.0 }
	pub fn take(self) -> Expr { self.0.take() }
}
