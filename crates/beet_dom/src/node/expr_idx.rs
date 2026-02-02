//! Expression index types for tracking nodes in template macros.
//!
//! This module provides [`ExprIdx`] for uniquely identifying expressions
//! within a template macro, enabling correlation between static analysis
//! and runtime instances.

use beet_core::prelude::*;

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
	/// Creates a new expression index with the given value.
	pub fn new(index: u32) -> Self { Self(index) }
}

impl std::fmt::Display for ExprIdx {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// Builder for assigning sequential [`ExprIdx`] values.
#[derive(Default)]
pub struct ExprIdxBuilder {
	current: u32,
}

impl ExprIdxBuilder {
	/// Creates a new builder starting at index 0.
	pub fn new() -> Self { Self::default() }

	/// Returns the next expression index and increments the counter.
	pub fn next(&mut self) -> ExprIdx {
		let idx = self.current;
		self.current += 1;
		ExprIdx::new(idx)
	}
}
