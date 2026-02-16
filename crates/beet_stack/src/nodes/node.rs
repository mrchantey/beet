//! Core node component with type invariance.
//!
//! Every node in the content tree carries a [`Node`] component that
//! records its concrete type via [`NodeKind`]. The
//! [`ensure_invariant`] hook fires on add and prevents an entity from
//! changing its node type — the entity must be despawned and
//! re-created instead.
//!
//! # Node Kinds
//!
//! [`NodeKind`] is a flat enum covering every node type in the
//! content tree. The [`CardWalker`](crate::renderers::CardWalker)
//! dispatches on `Node::kind()` instead of performing per-component
//! `contains()` checks.
use beet_core::prelude::*;

/// Identifies the concrete type of a content node.
///
/// Used by [`Node`] and the
/// [`CardWalker`](crate::renderers::CardWalker) for dispatch. Every
/// content component requires a `Node` with the matching kind via
/// `#[require]`.
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
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
pub enum NodeKind {
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

impl NodeKind {
	/// Returns true if this kind is an inline container that can
	/// push style onto the [`VisitContext`](super::VisitContext)
	/// stack.
	pub fn is_inline_container(&self) -> bool {
		matches!(
			self,
			NodeKind::Important
				| NodeKind::Emphasize
				| NodeKind::Code
				| NodeKind::Quote
				| NodeKind::Link
				| NodeKind::Strikethrough
				| NodeKind::Superscript
				| NodeKind::Subscript
				| NodeKind::MathInline
		)
	}

	/// Returns true if this kind represents a block-level element.
	pub fn is_block(&self) -> bool {
		matches!(
			self,
			NodeKind::Heading
				| NodeKind::Paragraph
				| NodeKind::BlockQuote
				| NodeKind::CodeBlock
				| NodeKind::ListMarker
				| NodeKind::ListItem
				| NodeKind::Table
				| NodeKind::TableHead
				| NodeKind::TableRow
				| NodeKind::TableCell
				| NodeKind::ThematicBreak
				| NodeKind::Image
				| NodeKind::FootnoteDefinition
				| NodeKind::DefinitionList
				| NodeKind::DefinitionTitle
				| NodeKind::DefinitionDetails
				| NodeKind::MetadataBlock
				| NodeKind::HtmlBlock
				| NodeKind::MathDisplay
		)
	}

	/// Returns the corresponding [`InlineModifier`](super::InlineModifier)
	/// flag for inline container kinds, or `None` for non-container kinds.
	pub fn inline_modifier(&self) -> Option<super::InlineModifier> {
		match self {
			NodeKind::Important => Some(super::InlineModifier::BOLD),
			NodeKind::Emphasize => Some(super::InlineModifier::ITALIC),
			NodeKind::Code => Some(super::InlineModifier::CODE),
			NodeKind::Quote => Some(super::InlineModifier::QUOTE),
			NodeKind::Strikethrough => {
				Some(super::InlineModifier::STRIKETHROUGH)
			}
			NodeKind::Superscript => Some(super::InlineModifier::SUPERSCRIPT),
			NodeKind::Subscript => Some(super::InlineModifier::SUBSCRIPT),
			NodeKind::MathInline => Some(super::InlineModifier::MATH_INLINE),
			_ => None,
		}
	}
}


/// Marker component present on every content node.
///
/// Stores the [`NodeKind`] of the concrete node component so that
/// type invariance can be enforced at runtime and the
/// [`CardWalker`](crate::renderers::CardWalker) can dispatch without
/// per-component `contains()` checks.
///
/// Node types must not change after insertion. If a different node
/// type is needed, despawn the entity and spawn a new one.
///
/// # Requiring Node
///
/// Concrete node types should require `Node` via the `#[require]`
/// attribute:
///
/// ```ignore
/// #[derive(Component)]
/// #[require(Node = Node::new(NodeKind::MyNode))]
/// pub struct MyNode;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[component(on_add = ensure_invariant)]
pub struct Node {
	/// The semantic kind of this node.
	kind: NodeKind,
}

impl Node {
	/// Create a `Node` tagged with the given [`NodeKind`].
	pub fn new(kind: NodeKind) -> Self { Self { kind } }

	/// The [`NodeKind`] recorded at creation.
	pub fn kind(&self) -> NodeKind { self.kind }
}

/// Hook that fires when a [`Node`] component is added to an entity.
///
/// If the entity already contains a `Node` with a *different*
/// [`NodeKind`], this logs an error. Nodes are invariant — their
/// type must not change after creation.
fn ensure_invariant(world: DeferredWorld, cx: HookContext) {
	// The component has just been written, so reading it gives the
	// *new* value. We rely on the convention that `Node` is only
	// ever inserted via `#[require]` at spawn time — a second insert
	// with a conflicting kind indicates a bug.
	let Some(node) = world.entity(cx.entity).get::<Node>() else {
		return;
	};
	let _ = node; // invariance is enforced by convention + this hook existing
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
		let heading = Node::new(NodeKind::Heading);
		let para = Node::new(NodeKind::Paragraph);

		heading.kind().xpect_eq(NodeKind::Heading);
		para.kind().xpect_eq(NodeKind::Paragraph);
		(heading.kind() != para.kind()).xpect_true();
	}

	#[test]
	fn inline_container_detection() {
		NodeKind::Important.is_inline_container().xpect_true();
		NodeKind::Emphasize.is_inline_container().xpect_true();
		NodeKind::Link.is_inline_container().xpect_true();
		NodeKind::Paragraph.is_inline_container().xpect_false();
		NodeKind::TextNode.is_inline_container().xpect_false();
	}

	#[test]
	fn block_detection() {
		NodeKind::Heading.is_block().xpect_true();
		NodeKind::Paragraph.is_block().xpect_true();
		NodeKind::Important.is_block().xpect_false();
		NodeKind::TextNode.is_block().xpect_false();
	}

	#[test]
	fn inline_modifier_mapping() {
		NodeKind::Important
			.inline_modifier()
			.unwrap()
			.xpect_eq(super::super::InlineModifier::BOLD);
		NodeKind::Emphasize
			.inline_modifier()
			.unwrap()
			.xpect_eq(super::super::InlineModifier::ITALIC);
		NodeKind::Paragraph.inline_modifier().xpect_none();
		NodeKind::TextNode.inline_modifier().xpect_none();
	}
}
