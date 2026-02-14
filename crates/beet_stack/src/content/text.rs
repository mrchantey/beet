//! Semantic text content primitives for interface-agnostic representation.
//!
//! This module provides components for describing text by its *meaning* rather
//! than its visual appearance. This semantic approach allows content to be
//! rendered appropriately across diverse interfaces.
//!
//! # Core Components
//!
//! - [`TextContent`] - The actual text string, always a child of a structural component
//! - [`FieldRef`](crate::document::FieldRef) - Dynamic binding to document fields
//!
//! # Text Structure
//!
//! Following the Bevy TextSpan pattern, text content is always a direct child
//! of its structural parent ([`Title`], [`Paragraph`], [`Link`]). Children
//! with [`TextContent`] extend the parent's text in sequence. Each child may
//! carry semantic markers like [`Important`] or [`Emphasize`].
//!
//! ```
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! // Simple title with text as a child
//! let title = Title::with_text("Hello World");
//!
//! // Paragraph with mixed static and dynamic content
//! let paragraph = (Paragraph, children![
//!     TextContent::new("The count is "),
//!     (Important, TextContent::new("42")),
//! ]);
//! ```
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
//! # Text Traversal
//!
//! Use [`TextQuery`] to collect text from structural elements
//! without manually walking the child tree. It handles inline
//! markers ([`Important`], [`Emphasize`], etc.) and respects
//! structural boundaries.
use beet_core::prelude::*;


/// A string of text, always used as a direct child of a structural
/// component like [`Title`] or [`Paragraph`].
///
/// If the entity also contains a
/// [`FieldRef`](crate::document::FieldRef), the text will be
/// automatically synchronized with the referenced field value.
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

/// Helper for types commonly constructed with a [`TextContent`] child.
///
/// Spawns the text as a child entity, following the TextSpan pattern
/// where content is always a direct child of its structural parent.
pub trait WithText: Default + Bundle + Clone {
	/// Create this component with a [`TextContent`] child.
	fn with_text(text: impl Into<String>) -> impl Bundle {
		(Self::default(), children![TextContent::new(text)])
	}
}

impl WithText for Title {}
impl WithText for Paragraph {}

/// Marker component used to denote a heading.
///
/// Text content is provided via [`TextContent`] children.
/// Nesting level is calculated and stored in [`TitleLevel`]
/// via an observer on insert.
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
#[require(TitleLevel)]
pub struct Title;


/// The computed nesting level of a [`Title`].
///
/// Level 0 is the main/root title. Each ancestor [`Title`]
/// (including via sibling parents) increments the level.
/// Calculated automatically via an observer on [`Title`] insert.
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
pub struct TitleLevel(pub u8);


/// Marker component to denote a paragraph of text.
///
/// Text content is provided via [`TextContent`] children.
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
pub struct Paragraph;

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
