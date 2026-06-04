//! Constraint trait, behaviors and validation errors used by [`ValueSchema`].
use crate::prelude::*;

/// A future returned by [`ApplyConstraints::apply`].
///
/// Carries the borrows for `self`, the path and the value, so apply impls can
/// freely borrow and recurse without `'static`.
pub type ApplyFuture<'a> =
	core::pin::Pin<Box<dyn 'a + Send + Future<Output = Vec<ValidationError>>>>;

/// Apply a constraint (or schema) to a value, producing validation errors.
///
/// The trait is async (returns a [`ApplyFuture`]) so I/O-bound validations
/// such as remote uniqueness checks can be implemented uniformly. Sync
/// constraints just return a ready future.
pub trait ApplyConstraints {
	/// The type of value this constraint can be applied to.
	type Value;
	/// Apply this constraint at the given path, possibly mutating `value`.
	fn apply<'a>(
		&'a self,
		path: &'a FieldPath,
		value: &'a mut Self::Value,
	) -> ApplyFuture<'a>;
}

/// How a constraint handles values that do not match.
#[derive(
	Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConstraintBehavior {
	/// Emit a [`ValidationError`].
	#[default]
	Error,
	/// Coerce the value to satisfy the constraint.
	Mutate,
}

/// An error produced by validating a [`Value`] against a [`ValueSchema`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidationError {
	/// The path within the root value where the error occurred.
	pub path: FieldPath,
	/// A human readable description of what failed.
	pub message: SmolStr,
}

impl ValidationError {
	/// Create a new validation error.
	pub fn new(path: FieldPath, message: impl Into<SmolStr>) -> Self {
		Self {
			path,
			message: message.into(),
		}
	}
}

impl core::fmt::Display for ValidationError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		if self.path.is_empty() {
			write!(f, "{}", self.message)
		} else {
			write!(f, "{}: {}", self.path, self.message)
		}
	}
}
