use beet_core::prelude::*;

/// Expressions are jsx-like components of a string, escaped by `{}`.
/// They may occur in rusty mdx or rusty jsx (rsx) flavors.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
pub struct Expression(pub String);
