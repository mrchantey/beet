//! Block-level and inline content components.
//!
//! Extends the core text primitives in [`super::text`] with structural
//! elements needed to represent full documents. Every component here
//! is interface-agnostic: renderers decide how to present them.
//!
//! # Block Elements
//!
//! - [`BlockQuote`] - quoted section
//! - [`CodeBlock`] - fenced or indented code listing
//! - [`ListMarker`] + [`ListItem`] - ordered and unordered list
//! - [`Table`], [`TableHead`], [`TableRow`], [`TableCell`] - tabular data
//! - [`ThematicBreak`] - section divider (horizontal rule)
//! - [`Image`] - image with alt text
//! - [`FootnoteDefinition`] - footnote body
//! - [`DefinitionList`], [`DefinitionTitle`], [`DefinitionDetails`]
//! - [`MetadataBlock`] - YAML/TOML frontmatter
//! - [`HtmlBlock`] - raw HTML block pass-through
//! - [`MathDisplay`] - display-mode math block
//!
//! # Inline Elements
//!
//! - [`Strikethrough`] - deleted or corrected text
//! - [`Superscript`] / [`Subscript`] - super/subscript text
//! - [`HardBreak`] / [`SoftBreak`] - line break within a block
//! - [`FootnoteRef`] - footnote reference marker
//! - [`MathInline`] - inline math
//! - [`HtmlInline`] - raw inline HTML pass-through
use super::DisplayBlock;
use super::DisplayInline;
use super::TextAlignment;
use super::node::Node;
use super::node::NodeKind;
use beet_core::prelude::*;


// ---------------------------------------------------------------------------
// Block-level components
// ---------------------------------------------------------------------------

/// A block-level quotation, semantically equivalent to HTML `<blockquote>`.
///
/// Children are structural elements ([`Paragraph`](super::Paragraph),
/// nested [`BlockQuote`], etc.).
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
#[require(DisplayBlock, Node = Node::new(NodeKind::BlockQuote))]
pub struct BlockQuote;


/// A fenced or indented code listing.
///
/// Semantically equivalent to HTML `<pre><code>`. The actual source
/// text is stored in [`TextNode`](super::TextNode) children; the
/// optional `language` field enables syntax highlighting.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(DisplayBlock, Node = Node::new(NodeKind::CodeBlock))]
pub struct CodeBlock {
	/// The language tag from the opening fence, if any.
	pub language: Option<String>,
}

impl CodeBlock {
	/// Create a code block with no language tag.
	pub fn plain() -> Self { Self { language: None } }

	/// Create a code block with the given language tag.
	pub fn with_language(language: impl Into<String>) -> Self {
		Self {
			language: Some(language.into()),
		}
	}
}


/// Container for an ordered or unordered list.
///
/// Children should be [`ListItem`] entities. For ordered list the
/// `start` field indicates the first item number.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect, Component,
)]
#[reflect(Component)]
#[require(DisplayBlock, Node = Node::new(NodeKind::ListMarker))]
pub struct ListMarker {
	/// Whether the list is ordered (numbered).
	pub ordered: bool,
	/// Starting number for ordered list, typically 1.
	pub start: Option<u64>,
}

impl ListMarker {
	/// Create an unordered (bullet) list marker.
	pub fn unordered() -> Self {
		Self {
			ordered: false,
			start: None,
		}
	}
	/// Create an ordered (numbered) list marker starting at `start`.
	pub fn ordered(start: u64) -> Self {
		Self {
			ordered: true,
			start: Some(start),
		}
	}
}


/// A single item within a [`ListMarker`] list.
///
/// Children are typically [`Paragraph`](super::Paragraph) or other
/// block elements.
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
#[require(DisplayBlock, Node = Node::new(NodeKind::ListItem))]
pub struct ListItem;


/// A section divider, semantically equivalent to HTML `<hr>`.
///
/// Typically rendered as a horizontal line separating content sections.
/// Has no children.
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
#[require(DisplayBlock, Node = Node::new(NodeKind::ThematicBreak))]
pub struct ThematicBreak;


/// An image element, semantically equivalent to HTML `<img>`.
///
/// Alt text is stored in [`TextNode`](super::TextNode) children,
/// following the same parent-child text pattern as other content.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(Node = Node::new(NodeKind::Image))]
pub struct Image {
	/// The image source URL or path.
	pub src: String,
	/// An optional title, often rendered as a tooltip.
	pub title: Option<String>,
}

impl Image {
	/// Create an image with the given source.
	pub fn new(src: impl Into<String>) -> Self {
		Self {
			src: src.into(),
			title: None,
		}
	}
	/// Set the title for this image.
	pub fn with_title(mut self, title: impl Into<String>) -> Self {
		self.title = Some(title.into());
		self
	}
}


/// A table container, semantically equivalent to HTML `<table>`.
///
/// Children should be [`TableHead`] and [`TableRow`] entities.
/// Column alignments are stored here for renderers to reference.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(DisplayBlock, Node = Node::new(NodeKind::Table))]
pub struct Table {
	/// Per-column alignment, indexed by column position.
	pub alignments: Vec<TextAlignment>,
}


/// The header row section of a [`Table`], equivalent to HTML `<thead>`.
///
/// Children should be [`TableCell`] entities with `header: true`.
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
#[require(Node = Node::new(NodeKind::TableHead))]
pub struct TableHead;


/// A single row within a [`Table`], equivalent to HTML `<tr>`.
///
/// Children should be [`TableCell`] entities.
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
#[require(Node = Node::new(NodeKind::TableRow))]
pub struct TableRow;


/// A single cell within a [`TableRow`] or [`TableHead`].
///
/// Text content follows the standard parent-child
/// [`TextNode`](super::TextNode) pattern.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect, Component,
)]
#[reflect(Component)]
#[require(Node = Node::new(NodeKind::TableCell))]
pub struct TableCell {
	/// Whether this cell is a header cell (`<th>` vs `<td>`).
	pub header: bool,
	/// Column alignment for this cell.
	pub alignment: TextAlignment,
}


/// A footnote body, equivalent to the target of a [`FootnoteRef`].
///
/// The `label` matches the reference marker. Children are the
/// footnote content (typically [`Paragraph`](super::Paragraph)).
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(DisplayBlock, Node = Node::new(NodeKind::FootnoteDefinition))]
pub struct FootnoteDefinition {
	/// The footnote label, matching the [`FootnoteRef::label`].
	pub label: String,
}


/// A definition list container, equivalent to HTML `<dl>`.
///
/// Children alternate between [`DefinitionTitle`] and
/// [`DefinitionDetails`] entities.
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
#[require(DisplayBlock, Node = Node::new(NodeKind::DefinitionList))]
pub struct DefinitionList;


/// A term being defined within a [`DefinitionList`], equivalent to
/// HTML `<dt>`.
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
#[require(Node = Node::new(NodeKind::DefinitionTitle))]
pub struct DefinitionTitle;


/// The definition body for a [`DefinitionTitle`], equivalent to
/// HTML `<dd>`.
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
#[require(Node = Node::new(NodeKind::DefinitionDetails))]
pub struct DefinitionDetails;


/// YAML or TOML frontmatter metadata block.
///
/// Stores the raw metadata string. Higher-level systems can
/// deserialize this into structured data as needed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(Node = Node::new(NodeKind::MetadataBlock))]
pub struct MetadataBlock {
	/// Whether the metadata is YAML or TOML.
	pub kind: MetadataKind,
	/// The raw metadata content between the delimiters.
	pub content: String,
}

/// The format of a [`MetadataBlock`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum MetadataKind {
	/// YAML frontmatter delimited by `---`.
	#[default]
	Yaml,
	/// TOML frontmatter delimited by `+++`.
	Toml,
}


/// Raw HTML block pass-through.
///
/// Stores the raw HTML string for renderers that support it.
/// Non-HTML renderers may choose to ignore or sanitize this.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(DisplayBlock, Node = Node::new(NodeKind::HtmlBlock))]
pub struct HtmlBlock(pub String);


/// Display-mode math block (equivalent to `$$...$$`).
///
/// The math expression is stored in [`TextNode`](super::TextNode)
/// children.
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
#[require(DisplayBlock, Node = Node::new(NodeKind::MathDisplay))]
pub struct MathDisplay;


// ---------------------------------------------------------------------------
// Inline components
// ---------------------------------------------------------------------------

/// Marker component for struck-through text.
///
/// Semantically equivalent to HTML `<del>` — text that has been
/// deleted or is no longer accurate.
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
#[require(Node = Node::new(NodeKind::Strikethrough), DisplayInline)]
pub struct Strikethrough;


/// Marker component for superscript text.
///
/// Semantically equivalent to HTML `<sup>`.
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
#[require(Node = Node::new(NodeKind::Superscript), DisplayInline)]
pub struct Superscript;


/// Marker component for subscript text.
///
/// Semantically equivalent to HTML `<sub>`.
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
#[require(Node = Node::new(NodeKind::Subscript), DisplayInline)]
pub struct Subscript;


/// A forced line break within a block, equivalent to HTML `<br>`.
///
/// In markdown this is produced by trailing two spaces or a backslash
/// at the end of a line.
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
#[require(Node = Node::new(NodeKind::HardBreak))]
pub struct HardBreak;


/// A soft line break, typically rendered as a space.
///
/// Produced by a single newline within a paragraph in the source
/// markdown.
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
#[require(Node = Node::new(NodeKind::SoftBreak))]
pub struct SoftBreak;


/// A footnote reference marker, equivalent to `[^label]` in markdown.
///
/// The referenced content lives in a [`FootnoteDefinition`] with a
/// matching label.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(Node = Node::new(NodeKind::FootnoteRef))]
pub struct FootnoteRef {
	/// The footnote label that links to a [`FootnoteDefinition`].
	pub label: String,
}


/// Inline math expression (equivalent to `$...$`).
///
/// The math expression string is stored in a
/// [`TextNode`](super::TextNode) on the same entity.
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
#[require(Node = Node::new(NodeKind::MathInline), DisplayInline)]
pub struct MathInline;


/// Raw inline HTML pass-through.
///
/// Non-HTML renderers may ignore or sanitize this content.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[require(Node = Node::new(NodeKind::HtmlInline))]
pub struct HtmlInline(pub String);
