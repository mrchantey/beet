//! Constraint policies used by [`ValueSchema`].
use crate::prelude::*;

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
