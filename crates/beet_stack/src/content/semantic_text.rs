use beet_core::prelude::*;

/// Text used to denote a heading.
///
/// Nesting is derived by the number of [`Title`] components
/// in ancestors.
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
pub struct Title(pub String);



/// A collection of sentences of unbounded length.
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
pub struct Paragraph(pub String);
