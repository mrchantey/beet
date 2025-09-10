#[cfg(feature = "tokens")]
use crate::prelude::*;
use bevy::prelude::*;

/// The index in which an expression appears in a template macro, assigned
/// in the order of the visitor that spawned it, ie rstml or rsx combinator.
/// Combining this with [`SnippetRoot`] we can uniquely identify
/// a template macro in a file, and the order of expressions inside it.
/// This is assigned to every node and attribute with a [`OnSpawnDeferred`]
#[derive(
	Debug, Copy, Clone, PartialEq, Eq, Hash, Deref, Reflect, Component,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
pub struct ExprIdx(pub u32);

impl ExprIdx {
	pub fn new(index: u32) -> Self { Self(index) }
}

impl std::fmt::Display for ExprIdx {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

#[derive(Default)]
pub struct ExprIdxBuilder {
	current: u32,
}
impl ExprIdxBuilder {
	pub fn new() -> Self { Self::default() }
	pub fn next(&mut self) -> ExprIdx {
		let idx = self.current;
		self.current += 1;
		ExprIdx::new(idx)
	}
}
