#[cfg(feature = "tokens")]
use crate::prelude::*;
use bevy::prelude::*;


/// An attribute belonging to the target entity, which may be
/// an element or a node.
#[derive(Component, Deref)]
#[relationship(relationship_target = Attributes)]
pub struct AttributeOf(Entity);

impl AttributeOf {
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// All attributes belonging to this entity, which may be
/// an element or a node.
#[derive(Component, Deref)]
#[relationship_target(relationship = AttributeOf)]
pub struct Attributes(Vec<Entity>);

/// An attribute key represented as a string.
///
/// ## Example
/// ```ignore
/// rsx!{<span "hidden"=true />};
/// ```#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeKeyStr(String);

impl AttributeKeyStr {
	pub fn new(value: String) -> Self { Self(value) }
	pub fn as_str(&self) -> &str { self.0.as_str() }
}


/// An attribute key represented as tokens, usually either a string literal or a block.
///
/// ## Example
/// ```ignore
/// let key = "hidden";
/// rsx!{<span {key}=true />};
/// ```
#[cfg(feature = "tokens")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeKeyExpr(NonSendHandle<syn::Expr>);
#[cfg(feature = "tokens")]
impl AttributeKeyExpr {
	pub fn new(value: NonSendHandle<syn::Expr>) -> Self { Self(value) }
}

/// The value of an attribute.
/// This defaults to the unit type `()` for convenience usage with helpers that disregard the type, ie [`FileSpanOf`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
pub struct AttributeValue<T = ()>(T);

impl<T> AttributeValue<T> {
	pub fn new(value: T) -> Self { Self(value) }
}

/// For literal attribute value types like strings, numbers, and booleans,
/// store the stringified version of the value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeValueStr(String);

impl AttributeValueStr {
	pub fn new(value: String) -> Self { Self(value) }
	pub fn as_str(&self) -> &str { self.0.as_str() }
}

/// The tokens for an attribute value, usually a block or a literal.
#[cfg(feature = "tokens")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeValueExpr(NonSendHandle<syn::Expr>);


#[cfg(feature = "tokens")]
impl AttributeValueExpr {
	pub fn new(value: NonSendHandle<syn::Expr>) -> Self { Self(value) }
}


/// Tokens for an attribute without a specified key-value pair.
/// This is known as the spread attribute in JSX, although rust
/// apis dont require the `...` prefix.
/// ## Example
/// ```ignore
/// rsx!{<span {props} />};
/// ```
#[cfg(feature = "tokens")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeExpr(NonSendHandle<syn::Expr>);


#[cfg(feature = "tokens")]
impl AttributeExpr {
	pub fn new(value: NonSendHandle<syn::Expr>) -> Self { Self(value) }
}
