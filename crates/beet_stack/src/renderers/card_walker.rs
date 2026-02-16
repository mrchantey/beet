//! Depth-first card traversal via the visitor pattern.
//!
//! [`CardWalker`] is a [`SystemParam`] that walks a card's entity
//! tree in depth-first order, dispatching to a [`CardVisitor`] for
//! each entity. The walker stops at child [`Card`] boundaries so
//! each visitor only sees entities belonging to a single card.
//!
//! # Visitor Pattern
//!
//! Implement [`CardVisitor`] to react to specific node types during
//! traversal. Default implementations return [`ControlFlow::Continue`]
//! so you only need to override the methods you care about.
//!
//! Returning [`ControlFlow::Break`] from any visit method skips that
//! entity's children (but continues with the next sibling).
//!
//! # Lifecycle
//!
//! For each entity the walker calls:
//! 1. `visit_*` — enter the node
//! 2. recurse into children (unless the visit returned `Break`)
//! 3. `leave_*` — exit the node (only for types that have a leave method)
//!
//! The `leave_*` methods are useful for renderers that need to close
//! tags, append trailing newlines, or pop style context after all
//! children have been visited.
//!
//! # Inline Style
//!
//! Text entities may carry inline formatting markers ([`Important`],
//! [`Emphasize`], [`Code`], [`Quote`], etc.). Rather than dispatching
//! separate visitor calls for each marker, the walker collects them
//! into an [`InlineStyle`] struct and passes it alongside the
//! [`TextNode`] in [`CardVisitor::visit_text`].
//!
//! # Example
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use std::ops::ControlFlow;
//!
//! struct TextCollector(String);
//!
//! impl CardVisitor for TextCollector {
//!     fn visit_text(
//!         &mut self,
//!         _entity: Entity,
//!         text: &TextNode,
//!         _style: &InlineStyle,
//!     ) -> ControlFlow<()> {
//!         self.0.push_str(text.as_str());
//!         ControlFlow::Continue(())
//!     }
//! }
//! ```
use crate::prelude::*;
use beet_core::prelude::*;
use std::ops::ControlFlow;

/// Inline formatting markers collected from entity components.
///
/// The walker constructs this from the components present on a
/// [`TextNode`] entity so visitors can apply formatting without
/// needing their own queries.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InlineStyle {
	/// Text has strong importance (`<strong>`).
	pub important: bool,
	/// Text has stress emphasis (`<em>`).
	pub emphasize: bool,
	/// Text is an inline code fragment (`<code>`).
	pub code: bool,
	/// Text is an inline quotation (`<q>`).
	pub quote: bool,
	/// Text has been struck through (`<del>`).
	pub strikethrough: bool,
	/// Text is superscript (`<sup>`).
	pub superscript: bool,
	/// Text is subscript (`<sub>`).
	pub subscript: bool,
	/// Text is inline math (`$...$`).
	pub math_inline: bool,
	/// The entity carries a [`Link`] component.
	pub link: Option<Link>,
}

impl InlineStyle {
	/// Returns true if no inline formatting is applied.
	pub fn is_plain(&self) -> bool {
		!self.important
			&& !self.emphasize
			&& !self.code
			&& !self.quote
			&& !self.strikethrough
			&& !self.superscript
			&& !self.subscript
			&& !self.math_inline
			&& self.link.is_none()
	}

	/// Merge two styles, combining flags with logical OR and
	/// preferring `other`'s link if present.
	///
	/// Used to inherit inline markers from ancestor containers
	/// (eg an [`Important`] parent entity) onto descendant
	/// [`TextNode`] entities.
	pub fn merge(&self, other: &Self) -> Self {
		Self {
			important: self.important || other.important,
			emphasize: self.emphasize || other.emphasize,
			code: self.code || other.code,
			quote: self.quote || other.quote,
			strikethrough: self.strikethrough || other.strikethrough,
			superscript: self.superscript || other.superscript,
			subscript: self.subscript || other.subscript,
			math_inline: self.math_inline || other.math_inline,
			link: other.link.clone().or_else(|| self.link.clone()),
		}
	}

	/// Construct an [`InlineStyle`] by inspecting the components on
	/// an [`EntityRef`].
	fn from_entity(entity_ref: &EntityRef) -> Self {
		Self {
			important: entity_ref.contains::<Important>(),
			emphasize: entity_ref.contains::<Emphasize>(),
			code: entity_ref.contains::<Code>(),
			quote: entity_ref.contains::<Quote>(),
			strikethrough: entity_ref.contains::<Strikethrough>(),
			superscript: entity_ref.contains::<Superscript>(),
			subscript: entity_ref.contains::<Subscript>(),
			math_inline: entity_ref.contains::<MathInline>(),
			link: entity_ref.get::<Link>().cloned(),
		}
	}
}

/// System parameter for depth-first traversal of a card's entity tree.
///
/// Uses [`CardQuery`] for boundary detection and [`EntityRef`] queries
/// to inspect node components on each entity.
#[derive(SystemParam)]
pub struct CardWalker<'w, 's> {
	card_query: CardQuery<'w, 's>,
	children_query: Query<'w, 's, &'static Children>,
	entity_query: Query<'w, 's, EntityRef<'static>>,
}

impl CardWalker<'_, '_> {
	/// Walk a card's entity tree depth-first, calling the visitor
	/// for each entity. Starts from the card root of `entity`.
	pub fn walk_card<V: CardVisitor>(&self, visitor: &mut V, entity: Entity) {
		let root = self.card_query.card_root(entity);
		self.walk_entity(visitor, root, root, &InlineStyle::default());
	}

	/// Walk from a specific entity without resolving the card root.
	pub fn walk_from<V: CardVisitor>(&self, visitor: &mut V, entity: Entity) {
		self.walk_entity(visitor, entity, entity, &InlineStyle::default());
	}

	/// Recursive depth-first walk. `root` is the walk origin, used
	/// to distinguish the starting entity from child card boundaries.
	///
	/// `inherited_style` carries inline formatting markers from
	/// ancestor containers (eg an [`Important`] parent) so that
	/// descendant [`TextNode`] entities inherit them.
	fn walk_entity<V: CardVisitor>(
		&self,
		visitor: &mut V,
		entity: Entity,
		root: Entity,
		inherited_style: &InlineStyle,
	) {
		let Ok(entity_ref) = self.entity_query.get(entity) else {
			return;
		};

		// Check if this entity is an inline container (has inline
		// markers but no TextNode). If so, merge its markers into
		// the inherited style for children.
		let entity_inline = InlineStyle::from_entity(&entity_ref);
		let is_inline_container =
			!entity_inline.is_plain() && !entity_ref.contains::<TextNode>();
		let child_style = if is_inline_container {
			inherited_style.merge(&entity_inline)
		} else {
			inherited_style.clone()
		};

		// Dispatch to the appropriate visitor method based on components
		let (flow, node_kind) =
			self.dispatch_visit(visitor, entity, &entity_ref, inherited_style);

		// If the visitor returned Break, skip this entity's children
		if flow.is_break() {
			// Still call the leave method even when breaking
			self.dispatch_leave(visitor, entity, node_kind);
			return;
		}

		// Recurse into children
		if let Ok(children) = self.children_query.get(entity) {
			for child in children.iter() {
				// Stop at card boundaries (unless it's the root itself)
				if child != root && self.card_query.is_card(child) {
					continue;
				}
				self.walk_entity(visitor, child, root, &child_style);
			}
		}

		// Leave callback after children have been visited
		self.dispatch_leave(visitor, entity, node_kind);
	}

	/// Dispatch to the most specific visitor method for the entity.
	/// Returns the control flow AND which node kind was matched so
	/// we can call the corresponding leave method later.
	///
	/// `inherited_style` is merged with the entity's own inline
	/// markers when dispatching [`CardVisitor::visit_text`].
	fn dispatch_visit<V: CardVisitor>(
		&self,
		visitor: &mut V,
		entity: Entity,
		entity_ref: &EntityRef,
		inherited_style: &InlineStyle,
	) -> (ControlFlow<()>, NodeKind) {
		// Block-level elements
		if let Some(heading) = entity_ref.get::<Heading>() {
			return (visitor.visit_heading(entity, heading), NodeKind::Heading);
		}
		if entity_ref.contains::<Paragraph>() {
			return (visitor.visit_paragraph(entity), NodeKind::Paragraph);
		}
		if entity_ref.contains::<BlockQuote>() {
			return (visitor.visit_block_quote(entity), NodeKind::BlockQuote);
		}
		if let Some(code_block) = entity_ref.get::<CodeBlock>() {
			return (
				visitor.visit_code_block(entity, code_block),
				NodeKind::CodeBlock,
			);
		}
		if let Some(list_marker) = entity_ref.get::<ListMarker>() {
			return (visitor.visit_list(entity, list_marker), NodeKind::List);
		}
		if entity_ref.contains::<ListItem>() {
			return (visitor.visit_list_item(entity), NodeKind::ListItem);
		}
		if let Some(table) = entity_ref.get::<Table>() {
			return (visitor.visit_table(entity, table), NodeKind::Table);
		}
		if entity_ref.contains::<TableHead>() {
			return (visitor.visit_table_head(entity), NodeKind::TableHead);
		}
		if entity_ref.contains::<TableRow>() {
			return (visitor.visit_table_row(entity), NodeKind::TableRow);
		}
		if entity_ref.contains::<TableCell>() {
			return (visitor.visit_table_cell(entity), NodeKind::TableCell);
		}
		if entity_ref.contains::<ThematicBreak>() {
			return (
				visitor.visit_thematic_break(entity),
				NodeKind::ThematicBreak,
			);
		}
		if let Some(image) = entity_ref.get::<Image>() {
			return (visitor.visit_image(entity, image), NodeKind::Image);
		}
		if let Some(footnote_def) = entity_ref.get::<FootnoteDefinition>() {
			return (
				visitor.visit_footnote_definition(entity, footnote_def),
				NodeKind::FootnoteDefinition,
			);
		}
		if entity_ref.contains::<MathDisplay>() {
			return (visitor.visit_math_display(entity), NodeKind::MathDisplay);
		}
		if let Some(html_block) = entity_ref.get::<HtmlBlock>() {
			return (
				visitor.visit_html_block(entity, html_block),
				NodeKind::HtmlBlock,
			);
		}

		// Button (form element)
		if entity_ref.contains::<Button>() {
			let text = entity_ref.get::<TextNode>().cloned();
			return (
				visitor.visit_button(entity, text.as_ref()),
				NodeKind::Button,
			);
		}

		// Inline elements — text with inline style merged from
		// ancestor containers and the entity's own markers
		if let Some(text) = entity_ref.get::<TextNode>() {
			let entity_style = InlineStyle::from_entity(entity_ref);
			let merged = inherited_style.merge(&entity_style);
			return (visitor.visit_text(entity, text, &merged), NodeKind::Text);
		}
		if let Some(link) = entity_ref.get::<Link>() {
			return (visitor.visit_link(entity, link), NodeKind::Link);
		}
		if entity_ref.contains::<HardBreak>() {
			return (visitor.visit_hard_break(entity), NodeKind::HardBreak);
		}
		if entity_ref.contains::<SoftBreak>() {
			return (visitor.visit_soft_break(entity), NodeKind::SoftBreak);
		}
		if let Some(footnote_ref) = entity_ref.get::<FootnoteRef>() {
			return (
				visitor.visit_footnote_ref(entity, footnote_ref),
				NodeKind::FootnoteRef,
			);
		}
		if let Some(html_inline) = entity_ref.get::<HtmlInline>() {
			return (
				visitor.visit_html_inline(entity, html_inline),
				NodeKind::HtmlInline,
			);
		}
		if let Some(task_check) = entity_ref.get::<TaskListCheck>() {
			return (
				visitor.visit_task_list_check(entity, task_check),
				NodeKind::TaskListCheck,
			);
		}

		// Fallback for any entity not matching a known node type
		(visitor.visit_entity(entity), NodeKind::Entity)
	}

	/// Dispatch the corresponding leave method for a previously
	/// visited node kind.
	fn dispatch_leave<V: CardVisitor>(
		&self,
		visitor: &mut V,
		entity: Entity,
		kind: NodeKind,
	) {
		match kind {
			NodeKind::Heading => visitor.leave_heading(entity),
			NodeKind::Paragraph => visitor.leave_paragraph(entity),
			NodeKind::BlockQuote => visitor.leave_block_quote(entity),
			NodeKind::CodeBlock => visitor.leave_code_block(entity),
			NodeKind::List => visitor.leave_list(entity),
			NodeKind::ListItem => visitor.leave_list_item(entity),
			NodeKind::Table => visitor.leave_table(entity),
			NodeKind::TableHead => visitor.leave_table_head(entity),
			NodeKind::TableRow => visitor.leave_table_row(entity),
			NodeKind::TableCell => visitor.leave_table_cell(entity),
			// Types without meaningful leave semantics
			NodeKind::ThematicBreak
			| NodeKind::Image
			| NodeKind::FootnoteDefinition
			| NodeKind::MathDisplay
			| NodeKind::HtmlBlock
			| NodeKind::Button
			| NodeKind::Text
			| NodeKind::Link
			| NodeKind::HardBreak
			| NodeKind::SoftBreak
			| NodeKind::FootnoteRef
			| NodeKind::HtmlInline
			| NodeKind::TaskListCheck
			| NodeKind::Entity => {}
		}
	}
}

/// Internal tag used to pair a `visit_*` call with its `leave_*`
/// counterpart without storing the full entity data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
	Entity,
	// Block-level
	Heading,
	Paragraph,
	BlockQuote,
	CodeBlock,
	List,
	ListItem,
	Table,
	TableHead,
	TableRow,
	TableCell,
	ThematicBreak,
	Image,
	FootnoteDefinition,
	MathDisplay,
	HtmlBlock,
	// Form
	Button,
	// Inline
	Text,
	Link,
	HardBreak,
	SoftBreak,
	FootnoteRef,
	HtmlInline,
	TaskListCheck,
}

/// Visitor trait for card tree traversal.
///
/// Each `visit_*` method corresponds to a node type in the content
/// tree. Default implementations return [`ControlFlow::Continue`], so
/// you only need to override the methods relevant to your use case.
///
/// Return [`ControlFlow::Break`] to skip traversing into the
/// visited entity's children (the corresponding `leave_*` method
/// is still called).
///
/// `leave_*` methods are called after all children have been visited.
/// They have no return value — traversal always continues to the
/// next sibling.
#[allow(unused_variables)]
pub trait CardVisitor {
	/// Called for entities that don't match any specific node type.
	fn visit_entity(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Block-level visit --

	/// Called when entering a [`Heading`] entity.
	fn visit_heading(
		&mut self,
		entity: Entity,
		heading: &Heading,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`Paragraph`] entity.
	fn visit_paragraph(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`BlockQuote`] entity.
	fn visit_block_quote(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`CodeBlock`] entity.
	fn visit_code_block(
		&mut self,
		entity: Entity,
		code_block: &CodeBlock,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`ListMarker`] entity.
	fn visit_list(
		&mut self,
		entity: Entity,
		list_marker: &ListMarker,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`ListItem`] entity.
	fn visit_list_item(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`Table`] entity.
	fn visit_table(
		&mut self,
		entity: Entity,
		table: &Table,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`TableHead`] entity.
	fn visit_table_head(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`TableRow`] entity.
	fn visit_table_row(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`TableCell`] entity.
	fn visit_table_cell(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`ThematicBreak`] entities (no children).
	fn visit_thematic_break(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`Image`] entities.
	fn visit_image(
		&mut self,
		entity: Entity,
		image: &Image,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`FootnoteDefinition`] entities.
	fn visit_footnote_definition(
		&mut self,
		entity: Entity,
		footnote_def: &FootnoteDefinition,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`MathDisplay`] entities.
	fn visit_math_display(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`HtmlBlock`] entities.
	fn visit_html_block(
		&mut self,
		entity: Entity,
		html_block: &HtmlBlock,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Form elements --

	/// Called for [`Button`] entities. The optional [`TextNode`] is
	/// the button label when present on the same entity.
	fn visit_button(
		&mut self,
		entity: Entity,
		label: Option<&TextNode>,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Inline --

	/// Called for [`TextNode`] entities. The [`InlineStyle`] carries
	/// formatting markers present on the same entity.
	fn visit_text(
		&mut self,
		entity: Entity,
		text: &TextNode,
		style: &InlineStyle,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`Link`] entities that do NOT also have a
	/// [`TextNode`] (standalone link containers). When a link is on
	/// the same entity as text, it appears in [`InlineStyle::link`]
	/// via [`visit_text`] instead.
	fn visit_link(&mut self, entity: Entity, link: &Link) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`HardBreak`] entities.
	fn visit_hard_break(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`SoftBreak`] entities.
	fn visit_soft_break(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`FootnoteRef`] entities.
	fn visit_footnote_ref(
		&mut self,
		entity: Entity,
		footnote_ref: &FootnoteRef,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`HtmlInline`] entities.
	fn visit_html_inline(
		&mut self,
		entity: Entity,
		html_inline: &HtmlInline,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`TaskListCheck`] entities.
	fn visit_task_list_check(
		&mut self,
		entity: Entity,
		task_check: &TaskListCheck,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Block-level leave --

	/// Called after leaving a [`Heading`] entity and all its children.
	fn leave_heading(&mut self, entity: Entity) {}

	/// Called after leaving a [`Paragraph`] entity and all its children.
	fn leave_paragraph(&mut self, entity: Entity) {}

	/// Called after leaving a [`BlockQuote`] entity and all its children.
	fn leave_block_quote(&mut self, entity: Entity) {}

	/// Called after leaving a [`CodeBlock`] entity and all its children.
	fn leave_code_block(&mut self, entity: Entity) {}

	/// Called after leaving a [`ListMarker`] entity and all its children.
	fn leave_list(&mut self, entity: Entity) {}

	/// Called after leaving a [`ListItem`] entity and all its children.
	fn leave_list_item(&mut self, entity: Entity) {}

	/// Called after leaving a [`Table`] entity and all its children.
	fn leave_table(&mut self, entity: Entity) {}

	/// Called after leaving a [`TableHead`] entity and all its children.
	fn leave_table_head(&mut self, entity: Entity) {}

	/// Called after leaving a [`TableRow`] entity and all its children.
	fn leave_table_row(&mut self, entity: Entity) {}

	/// Called after leaving a [`TableCell`] entity and all its children.
	fn leave_table_cell(&mut self, entity: Entity) {}
}


#[cfg(test)]
mod test {
	use super::*;

	struct EntityCounter(usize);

	impl CardVisitor for EntityCounter {
		fn visit_entity(&mut self, _entity: Entity) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
		fn visit_heading(
			&mut self,
			_entity: Entity,
			_heading: &Heading,
		) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
		fn visit_paragraph(&mut self, _entity: Entity) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
		fn visit_text(
			&mut self,
			_entity: Entity,
			_text: &TextNode,
			_style: &InlineStyle,
		) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
	}

	#[test]
	fn walks_card_tree() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![
				(Heading1, children![TextNode::new("Title")]),
				(Paragraph, children![TextNode::new("Body")]),
			]))
			.id();

		let count = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut counter = EntityCounter(0);
					walker.walk_card(&mut counter, entity);
					counter.0
				},
				card,
			)
			.unwrap();

		// card + heading + text + paragraph + text = 5
		count.xpect_eq(5);
	}

	#[test]
	fn stops_at_card_boundary() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![
				(Paragraph, children![TextNode::new("parent")]),
				(Card, children![(Paragraph, children![TextNode::new(
					"child card"
				)]),]),
			]))
			.id();

		let count = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut counter = EntityCounter(0);
					walker.walk_card(&mut counter, entity);
					counter.0
				},
				card,
			)
			.unwrap();

		// card + paragraph + text = 3 (child card is excluded)
		count.xpect_eq(3);
	}

	struct TextCollector(String);

	impl CardVisitor for TextCollector {
		fn visit_text(
			&mut self,
			_entity: Entity,
			text: &TextNode,
			_style: &InlineStyle,
		) -> ControlFlow<()> {
			self.0.push_str(text.as_str());
			ControlFlow::Continue(())
		}
	}

	#[test]
	fn collects_text() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![
				(Heading1, children![TextNode::new("Hello ")]),
				(Paragraph, children![TextNode::new("World")]),
			]))
			.id();

		let text = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut collector = TextCollector(String::new());
					walker.walk_card(&mut collector, entity);
					collector.0
				},
				card,
			)
			.unwrap();

		text.xpect_eq("Hello World");
	}

	#[test]
	fn break_skips_children() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![
				(Heading1, children![TextNode::new("Skipped")]),
				(Paragraph, children![TextNode::new("Not skipped")]),
			]))
			.id();

		// Walk with a text collector that also breaks on headings
		struct BreakHeadingTextCollector(String);
		impl CardVisitor for BreakHeadingTextCollector {
			fn visit_heading(
				&mut self,
				_entity: Entity,
				_heading: &Heading,
			) -> ControlFlow<()> {
				ControlFlow::Break(())
			}
			fn visit_text(
				&mut self,
				_entity: Entity,
				text: &TextNode,
				_style: &InlineStyle,
			) -> ControlFlow<()> {
				self.0.push_str(text.as_str());
				ControlFlow::Continue(())
			}
		}

		let text = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut collector =
						BreakHeadingTextCollector(String::new());
					walker.walk_card(&mut collector, entity);
					collector.0
				},
				card,
			)
			.unwrap();

		// Heading children skipped, only paragraph text collected
		text.xpect_eq("Not skipped");
	}

	#[test]
	fn inline_style_from_entity() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![(Paragraph, children![(
				Important,
				Emphasize,
				TextNode::new("styled")
			),])]))
			.id();

		struct StyleChecker(Vec<InlineStyle>);
		impl CardVisitor for StyleChecker {
			fn visit_text(
				&mut self,
				_entity: Entity,
				_text: &TextNode,
				style: &InlineStyle,
			) -> ControlFlow<()> {
				self.0.push(style.clone());
				ControlFlow::Continue(())
			}
		}

		let styles = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut checker = StyleChecker(Vec::new());
					walker.walk_card(&mut checker, entity);
					checker.0
				},
				card,
			)
			.unwrap();

		styles.len().xpect_eq(1);
		styles[0].important.xpect_true();
		styles[0].emphasize.xpect_true();
		styles[0].code.xpect_false();
	}

	#[test]
	fn leave_methods_called() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![(Heading1, children![TextNode::new(
				"Title"
			)]),]))
			.id();

		struct LifecycleTracker(Vec<String>);
		impl CardVisitor for LifecycleTracker {
			fn visit_heading(
				&mut self,
				_entity: Entity,
				heading: &Heading,
			) -> ControlFlow<()> {
				self.0.push(format!("enter_h{}", heading.level()));
				ControlFlow::Continue(())
			}
			fn visit_text(
				&mut self,
				_entity: Entity,
				text: &TextNode,
				_style: &InlineStyle,
			) -> ControlFlow<()> {
				self.0.push(format!("text:{}", text.as_str()));
				ControlFlow::Continue(())
			}
			fn leave_heading(&mut self, _entity: Entity) {
				self.0.push("leave_h".to_string());
			}
		}

		let events = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut tracker = LifecycleTracker(Vec::new());
					walker.walk_card(&mut tracker, entity);
					tracker.0
				},
				card,
			)
			.unwrap();

		events.xpect_eq(vec![
			"enter_h1".to_string(),
			"text:Title".to_string(),
			"leave_h".to_string(),
		]);
	}

	#[test]
	fn leave_called_even_on_break() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![(Heading1, children![TextNode::new(
				"Skipped"
			)]),]))
			.id();

		struct BreakTracker(Vec<String>);
		impl CardVisitor for BreakTracker {
			fn visit_heading(
				&mut self,
				_entity: Entity,
				_heading: &Heading,
			) -> ControlFlow<()> {
				self.0.push("enter_h".to_string());
				ControlFlow::Break(())
			}
			fn leave_heading(&mut self, _entity: Entity) {
				self.0.push("leave_h".to_string());
			}
		}

		let events = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut tracker = BreakTracker(Vec::new());
					walker.walk_card(&mut tracker, entity);
					tracker.0
				},
				card,
			)
			.unwrap();

		// leave_heading should still be called even though children were skipped
		events.xpect_eq(vec!["enter_h".to_string(), "leave_h".to_string()]);
	}

	#[test]
	fn plain_inline_style() {
		let style = InlineStyle::default();
		style.is_plain().xpect_true();
	}

	#[test]
	fn non_plain_inline_style() {
		let mut style = InlineStyle::default();
		style.important = true;
		style.is_plain().xpect_false();
	}

	#[test]
	fn link_inline_style() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![(Paragraph, children![(
				TextNode::new("click"),
				Link::new("https://example.com"),
			)])]))
			.id();

		struct LinkChecker(Vec<InlineStyle>);
		impl CardVisitor for LinkChecker {
			fn visit_text(
				&mut self,
				_entity: Entity,
				_text: &TextNode,
				style: &InlineStyle,
			) -> ControlFlow<()> {
				self.0.push(style.clone());
				ControlFlow::Continue(())
			}
		}

		let styles = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut checker = LinkChecker(Vec::new());
					walker.walk_card(&mut checker, entity);
					checker.0
				},
				card,
			)
			.unwrap();

		styles.len().xpect_eq(1);
		styles[0]
			.link
			.as_ref()
			.unwrap()
			.href
			.xpect_eq("https://example.com");
	}

	#[test]
	fn inherited_style_from_container() {
		// Mimics the markdown parser pattern where Important is a
		// container entity with TextNode children, rather than a
		// marker on the same entity as TextNode.
		let mut world = World::new();
		let card = world
			.spawn((Card, children![(Paragraph, children![
				TextNode::new("normal "),
				// Important as a CONTAINER with TextNode child
				(Important, children![TextNode::new("bold")]),
				TextNode::new(" end"),
			])]))
			.id();

		struct StyleCollector(Vec<(String, InlineStyle)>);
		impl CardVisitor for StyleCollector {
			fn visit_text(
				&mut self,
				_entity: Entity,
				text: &TextNode,
				style: &InlineStyle,
			) -> ControlFlow<()> {
				self.0.push((text.as_str().to_string(), style.clone()));
				ControlFlow::Continue(())
			}
		}

		let collected = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut collector = StyleCollector(Vec::new());
					walker.walk_card(&mut collector, entity);
					collector.0
				},
				card,
			)
			.unwrap();

		collected.len().xpect_eq(3);
		// "normal " should be plain
		collected[0].0.xpect_eq("normal ");
		collected[0].1.important.xpect_false();
		// "bold" should inherit Important from parent container
		collected[1].0.xpect_eq("bold");
		collected[1].1.important.xpect_true();
		// " end" should be plain again
		collected[2].0.xpect_eq(" end");
		collected[2].1.important.xpect_false();
	}

	#[test]
	fn nested_inherited_styles() {
		// Important container wrapping an Emphasize container
		let mut world = World::new();
		let card = world
			.spawn((Card, children![(Paragraph, children![(
				Important,
				children![(Emphasize, children![TextNode::new("bold italic")])],
			)])]))
			.id();

		struct StyleCollector(Vec<(String, InlineStyle)>);
		impl CardVisitor for StyleCollector {
			fn visit_text(
				&mut self,
				_entity: Entity,
				text: &TextNode,
				style: &InlineStyle,
			) -> ControlFlow<()> {
				self.0.push((text.as_str().to_string(), style.clone()));
				ControlFlow::Continue(())
			}
		}

		let collected = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut collector = StyleCollector(Vec::new());
					walker.walk_card(&mut collector, entity);
					collector.0
				},
				card,
			)
			.unwrap();

		collected.len().xpect_eq(1);
		collected[0].0.xpect_eq("bold italic");
		collected[0].1.important.xpect_true();
		collected[0].1.emphasize.xpect_true();
	}
}
