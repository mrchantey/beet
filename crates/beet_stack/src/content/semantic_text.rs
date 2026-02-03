//! Primitives for designing text according to its meaning.
//! This meaning may be interpreted by an interface for conveying
//! to the user. It is not nessecarily tied to User Interface, for instance
//! a voice assistant or humanoid robot may consume this content.
use beet_core::prelude::*;

/// A generic container for a string of text. This should
/// always have an associated semantic marker type,
/// like [`Title`] or [`Paragraph`]
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
pub struct TextContent(pub String);


/// Marker component used to denote a heading.
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
#[require(TextContent)]
pub struct Title;



/// Marker component to denote a paragraph of text.
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
#[require(TextContent)]
pub struct Paragraph;
