use crate::prelude::*;
use beet_core::prelude::*;


/// A tool that increments a specified field when triggered, returning the new value.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct Increment {
	/// Path to the field to increment.
	pub field: FieldRef,
}
