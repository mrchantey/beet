//! Semantic text content primitives for interface-agnostic representation.
//!
//! This module provides components for describing text by its *meaning* rather
//! than its visual appearance. This semantic approach allows content to be
//! rendered appropriately across diverse interfaces.
//!
//! # Core Components
//!
//! - [`TextContent`] - The actual text string
//! - [`TextBlock`] - Container for grouped text segments
//! - [`FieldRef`] - Dynamic binding to document fields
//!
//! # Semantic Markers
//!
//! - [`Important`] - Strong importance (like HTML `<strong>`)
//! - [`Emphasize`] - Stress emphasis (like HTML `<em>`)
//! - [`Code`] - Inline code fragment
//! - [`Quote`] - Inline quotation
//! - [`Link`] - Hyperlink to another resource
//!
//! # Structural Components
//!
//! - [`Title`] - Heading text (nesting derived from ancestors)
//! - [`Paragraph`] - Paragraph of text
//!
//! # Example
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! // Create a text block with mixed content
//! let block = text![
//!     "Welcome to ",
//!     (Important, "beet"),
//!     ", the ",
//!     (Emphasize, "semantic"),
//!     " framework!"
//! ];
//!
//! // Dynamic text bound to a document field
//! let dynamic = text![
//!     "Score: ",
//!     FieldRef::new("score").init_with(Value::I64(0))
//! ];
//! ```
use beet_core::prelude::*;


/// A generic container for a string of text.
/// These are often children of a semantic component like a [`Paragraph`]
/// If the entity contains
/// a [`FieldRef`], the text will be automatically synchronized
/// with the content of the [`FieldRef`].
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

impl TextContent {
	/// Create a new text content with the given string.
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }

	/// Returns the inner string as a slice.
	pub fn as_str(&self) -> &str { &self.0 }
}

impl<T: Into<String>> From<T> for TextContent {
	fn from(text: T) -> Self { Self(text.into()) }
}

/// Helper method added to types that are often
/// constructed alongside a [`TextContent`]
pub trait WithText: Default {
	fn with_text(text: impl Into<String>) -> (Self, TextContent) {
		(Self::default(), TextContent::new(text))
	}
}

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

impl WithText for Title {}
impl WithText for Paragraph {}


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

impl Paragraph {
	pub fn text(text: impl Into<String>) -> (Self, TextContent) {
		(Paragraph, TextContent::new(text))
	}
}

/// Marker component for important/strong text.
///
/// Semantically equivalent to HTML `<strong>` - text of strong importance.
/// Interfaces may render this as bold, louder speech, or other emphasis.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct Important;


/// Marker component for emphasized text.
///
/// Semantically equivalent to HTML `<em>` - stress emphasis.
/// Interfaces may render this as italics, altered pitch, or other emphasis.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct Emphasize;


/// Marker component for inline code or monospace text.
///
/// Semantically equivalent to HTML `<code>` - a fragment of computer code.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct Code;


/// Marker component for quoted text.
///
/// Semantically equivalent to HTML `<q>` - an inline quotation.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct Quote;


/// Component for hyperlink text.
///
/// Semantically equivalent to HTML `<a>` - a hyperlink to another resource.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(TextContent)]
pub struct Link {
	/// The URL this link points to.
	pub href: String,
	/// An optional title for the link, which may be used as tooltip text.
	/// The actual rendered text is the [`TextContent`].
	pub title: Option<String>,
}

impl Link {
	/// Create a new link with the given URL.
	pub fn new(href: impl Into<String>) -> Self {
		Self {
			href: href.into(),
			title: None,
		}
	}
	/// Set the title for this link.
	pub fn with_title(mut self, title: impl Into<String>) -> Self {
		self.title = Some(title.into());
		self
	}
}
