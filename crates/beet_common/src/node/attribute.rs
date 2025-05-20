#[cfg(feature = "tokens")]
use crate::as_beet::*;
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, DerefMut, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct AttributeKeyStr(pub String);

impl AttributeKeyStr {
	pub fn new(value: String) -> Self { Self(value) }
}


/// The key of an attribute, ususally a string literal but can be anything.
/// ## Example
/// ```ignore
/// rsx!{<span "hidden"=true {32}=true />};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, DerefMut, Component)]
pub struct AttributeKey<T>(pub T);

impl<T> AttributeKey<T> {
	pub fn new(key: T) -> Self { Self(key) }
}

/// The value of an attribute
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, DerefMut, Component)]
pub struct AttributeValue<T>(pub T);

impl<T> AttributeValue<T> {
	pub fn new(value: T) -> Self { Self(value) }
}

/// For literal attribute value types like strings, numbers, and booleans,
/// store the stringified version of the value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, DerefMut, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct AttributeValueStr(pub String);

impl AttributeValueStr {
	pub fn new(value: String) -> Self { Self(value) }
}
