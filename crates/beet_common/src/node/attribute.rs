#[cfg(feature = "tokens")]
use crate::as_beet::*;
use bevy::prelude::*;


/// An attribute belonging to the target entity, which may be
/// an element or a node.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Attributes)]
pub struct AttributeOf(Entity);

impl AttributeOf {
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// All attributes belonging to this entity, which may be
/// an element or a node.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = AttributeOf,linked_spawn)]
pub struct Attributes(Vec<Entity>);

/// An attribute key represented as a string.
///
/// ## Example
/// ```ignore
/// rsx!{<span "hidden"=true />};
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, Deref, DerefMut, Reflect, Component,
)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct AttributeKey(pub String);

impl AttributeKey {
	pub fn new(value: impl Into<String>) -> Self { Self(value.into()) }
}


/// For literal attribute value types like strings, numbers, and booleans,
/// store the stringified version of the value.
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, Deref, DerefMut, Reflect, Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct AttributeValueStr(pub String);

impl AttributeValueStr {
	pub fn new(value: impl Into<String>) -> Self { Self(value.into()) }
}

impl AsRef<str> for AttributeValueStr {
	fn as_ref(&self) -> &str { &self.0 }
}
