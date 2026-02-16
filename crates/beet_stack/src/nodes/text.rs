//! Semantic text content primitives for interface-agnostic representation.
//!
//! This module provides components for describing text by its *meaning* rather
//! than its visual appearance. This semantic approach allows content to be
//! rendered appropriately across diverse interfaces.
//!
//! # Core Components
//!
//! - [`TextNode`] - The actual text string, always a child of a structural component
//! - [`FieldRef`](crate::document::FieldRef) - Dynamic binding to document fields
//!
//! # Text Structure
//!
//! Following the Bevy TextSpan pattern, text content is always a direct child
//! of its structural parent ([`Heading1`], [`Paragraph`], [`Link`]). Children
//! with [`TextNode`] extend the parent's text in sequence. Each child may
//! carry inline node components like [`Important`] or [`Emphasize`].
//!
//! ```
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! // Simple heading with text as a child
//! let heading = Heading1::with_text("Hello World");
//!
//! // Paragraph with mixed static and dynamic content
//! let paragraph = (Paragraph, children![
//!     TextNode::new("The count is "),
//!     (Important, TextNode::new("42")),
//! ]);
//! ```
//!
//! # Inline Nodes
//!
//! These are proper nodes with [`Node`] and [`DisplayInline`]
//! requirements, semantically equivalent to their HTML counterparts:
//!
//! - [`Important`] - Strong importance (like HTML `<strong>`)
//! - [`Emphasize`] - Stress emphasis (like HTML `<em>`)
//! - [`Code`] - Inline code fragment (like HTML `<code>`)
//! - [`Quote`] - Inline quotation (like HTML `<q>`)
//! - [`Link`] - Hyperlink to another resource (like HTML `<a>`)
//!
//! # Structural Components
//!
//! - [`Heading1`]..=[`Heading6`] - Heading text at explicit levels
//! - [`Paragraph`] - Paragraph of text
//!
//! Both headings and paragraphs are [`DisplayBlock`] elements.
//!
//! # Text Traversal
//!
//! Use [`CardWalker`](crate::renderers::CardWalker) and the
//! [`CardVisitor`](crate::renderers::CardVisitor) trait to traverse
//! content trees. The walker dispatches to visitor methods for each
//! node type, including inline nodes, and handles structural
//! boundaries automatically.
use super::node::Node;
use super::node::NodeKind;
use crate::nodes::DisplayBlock;
use crate::nodes::DisplayInline;
use beet_core::prelude::*;


/// A string of text, always used as a direct child of a structural
/// component like [`Heading1`] or [`Paragraph`].
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
#[require(Node = Node::new(NodeKind::TextNode))]
pub struct TextNode(pub String);

impl TextNode {
	/// Create a new text content with the given string.
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }

	/// Returns the inner string as a slice.
	pub fn as_str(&self) -> &str { &self.0 }
}

impl<T: Into<String>> From<T> for TextNode {
	fn from(text: T) -> Self { Self(text.into()) }
}

/// Helper for types commonly constructed with a [`TextNode`] child.
///
/// Spawns the text as a child entity, following the TextSpan pattern
/// where content is always a direct child of its structural parent.
pub trait WithText: Default + Bundle + Clone {
	/// Create this component with a [`TextNode`] child.
	fn with_text(text: impl Into<String>) -> impl Bundle {
		(Self::default(), children![TextNode::new(text)])
	}
}

impl WithText for Heading1 {}
impl WithText for Heading2 {}
impl WithText for Heading3 {}
impl WithText for Heading4 {}
impl WithText for Heading5 {}
impl WithText for Heading6 {}
impl WithText for Paragraph {}


/// The heading level of a structural element.
///
/// Cannot be constructed directly; instead use [`Heading1`]..=[`Heading6`]
/// which require `Heading` at the appropriate level via the `#[require]`
/// attribute.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
#[reflect(Component)]
#[require(DisplayBlock)]
pub struct Heading {
	// private so only Heading1 etc can construct a Heading
	level: u8,
}

impl Heading {
	/// The heading level (1–6).
	pub fn level(&self) -> u8 { self.level }

	/// Level-1 heading, equivalent to HTML `<h1>`.
	fn new_level_one() -> Self { Self { level: 1 } }
	/// Level-2 heading, equivalent to HTML `<h2>`.
	fn new_level_two() -> Self { Self { level: 2 } }
	/// Level-3 heading, equivalent to HTML `<h3>`.
	fn new_level_three() -> Self { Self { level: 3 } }
	/// Level-4 heading, equivalent to HTML `<h4>`.
	fn new_level_four() -> Self { Self { level: 4 } }
	/// Level-5 heading, equivalent to HTML `<h5>`.
	fn new_level_five() -> Self { Self { level: 5 } }
	/// Level-6 heading, equivalent to HTML `<h6>`.
	fn new_level_six() -> Self { Self { level: 6 } }
}

impl Default for Heading {
	fn default() -> Self { Self::new_level_one() }
}


/// Level-1 heading, equivalent to HTML `<h1>`.
///
/// Text content is provided via [`TextNode`] children.
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
#[require(Heading = Heading::new_level_one(), Node = Node::new(NodeKind::Heading))]
pub struct Heading1;

/// Level-2 heading, equivalent to HTML `<h2>`.
///
/// Text content is provided via [`TextNode`] children.
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
#[require(Heading = Heading::new_level_two(), Node = Node::new(NodeKind::Heading))]
pub struct Heading2;

/// Level-3 heading, equivalent to HTML `<h3>`.
///
/// Text content is provided via [`TextNode`] children.
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
#[require(Heading = Heading::new_level_three(), Node = Node::new(NodeKind::Heading))]
pub struct Heading3;

/// Level-4 heading, equivalent to HTML `<h4>`.
///
/// Text content is provided via [`TextNode`] children.
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
#[require(Heading = Heading::new_level_four(), Node = Node::new(NodeKind::Heading))]
pub struct Heading4;

/// Level-5 heading, equivalent to HTML `<h5>`.
///
/// Text content is provided via [`TextNode`] children.
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
#[require(Heading = Heading::new_level_five(), Node = Node::new(NodeKind::Heading))]
pub struct Heading5;

/// Level-6 heading, equivalent to HTML `<h6>`.
///
/// Text content is provided via [`TextNode`] children.
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
#[require(Heading = Heading::new_level_six(), Node = Node::new(NodeKind::Heading))]
pub struct Heading6;


/// Marker component to denote a paragraph of text.
///
/// Text content is provided via [`TextNode`] children.
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
#[require(DisplayBlock, Node = Node::new(NodeKind::Paragraph))]
pub struct Paragraph;

/// Inline node for important/strong text.
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
#[require(Node = Node::new(NodeKind::Important), DisplayInline)]
pub struct Important;


/// Inline node for emphasized text.
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
#[require(Node = Node::new(NodeKind::Emphasize), DisplayInline)]
pub struct Emphasize;


/// Inline node for code or monospace text.
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
#[require(Node = Node::new(NodeKind::Code), DisplayInline)]
pub struct Code;


/// Inline node for quoted text.
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
#[require(Node = Node::new(NodeKind::Quote), DisplayInline)]
pub struct Quote;


/// Inline node for hyperlink text.
///
/// Semantically equivalent to HTML `<a>` - a hyperlink to another resource.
/// The link text can be either a [`TextNode`] on the same entity or
/// in child entities (the container pattern used by the markdown parser).
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(Node = Node::new(NodeKind::Link), DisplayInline)]
pub struct Link {
	/// The URL this link points to.
	pub href: String,
	/// An optional title for the link, which may be used as tooltip text.
	/// The actual rendered text is the [`TextNode`].
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
