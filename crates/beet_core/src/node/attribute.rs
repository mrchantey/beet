#[cfg(feature = "tokens")]
use crate::as_beet::*;
use bevy::prelude::*;


/// An attribute belonging to the target entity, which may be
/// an element or a node.
#[derive(Debug, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Attributes)]
pub struct AttributeOf(Entity);

impl AttributeOf {
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// All attributes belonging to this entity, which may be
/// an element or a template.
#[derive(Debug, Deref, Reflect, Component)]
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


/// For literal attribute value types like strings, numbers, and booleans
///
/// ## Hash
/// This type implements `Hash` including its f64 variant,
/// disregarding the fact that technically NaN is not equal to itself.
#[derive(Debug, Clone, PartialEq, Reflect, Component)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum AttributeLit {
	String(String),
	Number(f64),
	Boolean(bool),
}

impl AttributeLit {
	pub fn new(value: impl Into<Self>) -> Self { value.into() }
}
impl std::hash::Hash for AttributeLit {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		match self {
			Self::String(s) => s.hash(state),
			Self::Number(n) => n.to_string().hash(state),
			Self::Boolean(b) => b.hash(state),
		}
	}
}
impl std::fmt::Display for AttributeLit {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::String(s) => write!(f, "{}", s),
			Self::Number(n) => write!(f, "{}", n),
			Self::Boolean(b) => write!(f, "{}", b),
		}
	}
}

impl Into<AttributeLit> for String {
	fn into(self) -> AttributeLit { AttributeLit::String(self) }
}
impl Into<AttributeLit> for &String {
	fn into(self) -> AttributeLit { AttributeLit::String(self.clone()) }
}
impl Into<AttributeLit> for &str {
	fn into(self) -> AttributeLit { AttributeLit::String(self.to_string()) }
}

impl Into<AttributeLit> for bool {
	fn into(self) -> AttributeLit { AttributeLit::Boolean(self) }
}

impl Into<AttributeLit> for f32 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for f64 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self) }
}

impl Into<AttributeLit> for u8 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for u16 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for u32 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for u64 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for u128 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for usize {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for i8 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for i16 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for i32 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for i64 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for i128 {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}

impl Into<AttributeLit> for isize {
	fn into(self) -> AttributeLit { AttributeLit::Number(self as f64) }
}
