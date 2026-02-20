//! Core node component with type invariance.
//!
//! Every node in the content tree carries a [`Node`] component that
//! identifies its concrete type. The [`ensure_invariant`] hook fires
//! on add and prevents an entity from changing its node type — the
//! entity must be despawned and re-created instead.
//!
//! [`Node`] is a flat enum covering every node type in the content
//! tree. The [`CardWalker`](crate::renderers::CardWalker) dispatches
//! on `Node` variants instead of performing per-component
//! `contains()` checks.
use beet_core::prelude::*;

/// Identifies the concrete type of a content node.
///
/// Used by the [`CardWalker`](crate::renderers::CardWalker) for
/// dispatch. Every content component requires a `Node` with the
/// matching variant via `#[require]`.
///
/// Nodes are invariant — they must not change type after creation.
/// If a different node type is needed, despawn the entity and spawn
/// a new one.
///
/// # Requiring Node
///
/// Concrete node types should require `Node` via the `#[require]`
/// attribute:
///
/// ```ignore
/// #[derive(Component)]
/// #[require(Node = Node::MyNode)]
/// pub struct MyNode;
/// ```
///
/// # Block-level
///
/// Structural elements that start a new block of content.
///
/// # Inline containers
///
/// Wrapper elements that apply formatting to descendant
/// [`TextNode`](super::TextNode) children. Following the HTML model,
/// inline containers and [`TextNode`](super::TextNode) are mutually
/// exclusive on the same entity.
///
/// # Inline leaves
///
/// Leaf elements that appear within a block but carry no children
/// (breaks, references, raw HTML).
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
pub enum Node {
	// -- Block-level --
	/// Level 1–6 heading.
	Heading,
	/// Paragraph of text.
	#[default]
	Paragraph,
	/// Block quotation.
	BlockQuote,
	/// Fenced or indented code listing.
	CodeBlock,
	/// Ordered or unordered list container.
	ListMarker,
	/// Single item within a list.
	ListItem,
	/// Table container.
	Table,
	/// Table header section.
	TableHead,
	/// Table body row.
	TableRow,
	/// Table cell.
	TableCell,
	/// Horizontal rule / section divider.
	ThematicBreak,
	/// Image element.
	Image,
	/// Footnote body.
	FootnoteDefinition,
	/// Definition list container.
	DefinitionList,
	/// Term being defined.
	DefinitionTitle,
	/// Definition body.
	DefinitionDetails,
	/// YAML/TOML frontmatter.
	MetadataBlock,
	/// Raw HTML block.
	HtmlBlock,
	/// Display-mode math block.
	MathDisplay,

	// -- Form --
	/// Clickable action trigger.
	Button,
	/// Task list checkbox.
	TaskListCheck,

	// -- Text --
	/// String of text content.
	TextNode,

	// -- Inline containers --
	/// Strong importance (`<strong>`).
	Important,
	/// Stress emphasis (`<em>`).
	Emphasize,
	/// Inline code fragment (`<code>`).
	Code,
	/// Inline quotation (`<q>`).
	Quote,
	/// Hyperlink (`<a>`).
	Link,
	/// Struck-through text (`<del>`).
	Strikethrough,
	/// Superscript text (`<sup>`).
	Superscript,
	/// Subscript text (`<sub>`).
	Subscript,
	/// Inline math (`$...$`).
	MathInline,

	// -- Inline leaves --
	/// Forced line break (`<br>`).
	HardBreak,
	/// Soft line break (rendered as space).
	SoftBreak,
	/// Footnote reference marker.
	FootnoteRef,
	/// Raw inline HTML.
	HtmlInline,
}

impl Node {
	/// Returns true if this node is an inline container that can
	/// push style onto the [`VisitContext`](super::VisitContext)
	/// stack.
	pub fn is_inline_container(&self) -> bool {
		matches!(
			self,
			Node::Important
				| Node::Emphasize
				| Node::Code | Node::Quote
				| Node::Link | Node::Strikethrough
				| Node::Superscript
				| Node::Subscript
				| Node::MathInline
		)
	}

	/// Returns true if this node represents a block-level element.
	pub fn is_block(&self) -> bool {
		matches!(
			self,
			Node::Heading
				| Node::Paragraph
				| Node::BlockQuote
				| Node::CodeBlock
				| Node::ListMarker
				| Node::ListItem
				| Node::Table
				| Node::TableHead
				| Node::TableRow
				| Node::TableCell
				| Node::ThematicBreak
				| Node::Image
				| Node::FootnoteDefinition
				| Node::DefinitionList
				| Node::DefinitionTitle
				| Node::DefinitionDetails
				| Node::MetadataBlock
				| Node::HtmlBlock
				| Node::MathDisplay
		)
	}

	/// Returns the corresponding [`InlineStyle`](super::InlineStyle)
	/// flag for inline container nodes, or `None` for non-container nodes.
	pub fn inline_style(&self) -> Option<super::InlineStyle> {
		match self {
			Node::Important => Some(super::InlineStyle::BOLD),
			Node::Emphasize => Some(super::InlineStyle::ITALIC),
			Node::Code => Some(super::InlineStyle::CODE),
			Node::Quote => Some(super::InlineStyle::QUOTE),
			Node::Strikethrough => Some(super::InlineStyle::STRIKETHROUGH),
			Node::Superscript => Some(super::InlineStyle::SUPERSCRIPT),
			Node::Subscript => Some(super::InlineStyle::SUBSCRIPT),
			Node::MathInline => Some(super::InlineStyle::MATH_INLINE),
			Node::Link => Some(super::InlineStyle::LINK),
			_ => None,
		}
	}
}

/// Propagates [`TextNode`](super::TextNode) changes to the parent
/// [`Node`] component.
///
/// When a child [`TextNode`](super::TextNode) is modified, this
/// system marks the parent's [`Node`] component as changed so
/// downstream systems can react via `Changed<Node>`.
pub fn mark_node_changed(
	mut nodes: Query<&mut Node>,
	content: Query<&ChildOf, Changed<super::TextNode>>,
) {
	for child_of in &content {
		if let Ok(mut node) = nodes.get_mut(child_of.parent()) {
			node.set_changed();
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn node_records_kind() {
		let heading = Node::Heading;
		let para = Node::Paragraph;

		heading.xpect_eq(Node::Heading);
		para.xpect_eq(Node::Paragraph);
		(heading != para).xpect_true();
	}

	#[test]
	fn inline_container_detection() {
		Node::Important.is_inline_container().xpect_true();
		Node::Emphasize.is_inline_container().xpect_true();
		Node::Link.is_inline_container().xpect_true();
		Node::Paragraph.is_inline_container().xpect_false();
		Node::TextNode.is_inline_container().xpect_false();
	}

	#[test]
	fn block_detection() {
		Node::Heading.is_block().xpect_true();
		Node::Paragraph.is_block().xpect_true();
		Node::Important.is_block().xpect_false();
		Node::TextNode.is_block().xpect_false();
	}

	#[test]
	fn inline_style_mapping() {
		Node::Important
			.inline_style()
			.unwrap()
			.xpect_eq(super::super::InlineStyle::BOLD);
		Node::Emphasize
			.inline_style()
			.unwrap()
			.xpect_eq(super::super::InlineStyle::ITALIC);
		Node::Paragraph.inline_style().xpect_none();
		Node::TextNode.inline_style().xpect_none();
	}
}
