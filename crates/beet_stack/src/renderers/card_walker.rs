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
//! # Example
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use std::ops::ControlFlow;
//!
//! struct TextCollector(String);
//!
//! impl CardVisitor for TextCollector {
//!     fn visit_text(&mut self, _entity: Entity, text: &TextNode) -> ControlFlow<()> {
//!         self.0.push_str(text.as_str());
//!         ControlFlow::Continue(())
//!     }
//! }
//! ```
use crate::prelude::*;
use beet_core::prelude::*;
use std::ops::ControlFlow;

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
		self.walk_entity(visitor, root, root);
	}

	/// Walk from a specific entity without resolving the card root.
	pub fn walk_from<V: CardVisitor>(&self, visitor: &mut V, entity: Entity) {
		self.walk_entity(visitor, entity, entity);
	}

	/// Recursive depth-first walk. `root` is the walk origin, used
	/// to distinguish the starting entity from child card boundaries.
	fn walk_entity<V: CardVisitor>(
		&self,
		visitor: &mut V,
		entity: Entity,
		root: Entity,
	) {
		let Ok(entity_ref) = self.entity_query.get(entity) else {
			return;
		};

		// Dispatch to the appropriate visitor method based on components
		let flow = self.dispatch_visit(visitor, entity, &entity_ref);

		// If the visitor returned Break, skip this entity's children
		if flow.is_break() {
			return;
		}

		// Recurse into children
		let Ok(children) = self.children_query.get(entity) else {
			return;
		};

		for child in children.iter() {
			// Stop at card boundaries (unless it's the root itself)
			if child != root && self.card_query.is_card(child) {
				continue;
			}
			self.walk_entity(visitor, child, root);
		}
	}

	/// Dispatch to the most specific visitor method for the entity.
	fn dispatch_visit<V: CardVisitor>(
		&self,
		visitor: &mut V,
		entity: Entity,
		entity_ref: &EntityRef,
	) -> ControlFlow<()> {
		// Block-level elements
		if let Some(heading) = entity_ref.get::<Heading>() {
			return visitor.visit_heading(entity, heading);
		}
		if entity_ref.contains::<Paragraph>() {
			return visitor.visit_paragraph(entity);
		}
		if entity_ref.contains::<BlockQuote>() {
			return visitor.visit_block_quote(entity);
		}
		if let Some(code_block) = entity_ref.get::<CodeBlock>() {
			return visitor.visit_code_block(entity, code_block);
		}
		if let Some(list_marker) = entity_ref.get::<ListMarker>() {
			return visitor.visit_list(entity, list_marker);
		}
		if entity_ref.contains::<ListItem>() {
			return visitor.visit_list_item(entity);
		}
		if let Some(table) = entity_ref.get::<Table>() {
			return visitor.visit_table(entity, table);
		}
		if entity_ref.contains::<TableHead>() {
			return visitor.visit_table_head(entity);
		}
		if entity_ref.contains::<TableRow>() {
			return visitor.visit_table_row(entity);
		}
		if entity_ref.contains::<TableCell>() {
			return visitor.visit_table_cell(entity);
		}
		if entity_ref.contains::<ThematicBreak>() {
			return visitor.visit_thematic_break(entity);
		}
		if let Some(image) = entity_ref.get::<Image>() {
			return visitor.visit_image(entity, image);
		}
		if let Some(footnote_def) = entity_ref.get::<FootnoteDefinition>() {
			return visitor.visit_footnote_definition(entity, footnote_def);
		}
		if entity_ref.contains::<MathDisplay>() {
			return visitor.visit_math_display(entity);
		}
		if let Some(html_block) = entity_ref.get::<HtmlBlock>() {
			return visitor.visit_html_block(entity, html_block);
		}

		// Inline elements
		if let Some(text) = entity_ref.get::<TextNode>() {
			return visitor.visit_text(entity, text);
		}
		if let Some(link) = entity_ref.get::<Link>() {
			return visitor.visit_link(entity, link);
		}
		if entity_ref.contains::<HardBreak>() {
			return visitor.visit_hard_break(entity);
		}
		if entity_ref.contains::<SoftBreak>() {
			return visitor.visit_soft_break(entity);
		}
		if let Some(footnote_ref) = entity_ref.get::<FootnoteRef>() {
			return visitor.visit_footnote_ref(entity, footnote_ref);
		}
		if let Some(html_inline) = entity_ref.get::<HtmlInline>() {
			return visitor.visit_html_inline(entity, html_inline);
		}
		if let Some(task_check) = entity_ref.get::<TaskListCheck>() {
			return visitor.visit_task_list_check(entity, task_check);
		}

		// Fallback for any entity not matching a known node type
		visitor.visit_entity(entity)
	}
}

/// Visitor trait for card tree traversal.
///
/// Each method corresponds to a node type in the content tree.
/// Default implementations return [`ControlFlow::Continue`], so you
/// only need to override the methods relevant to your use case.
///
/// Return [`ControlFlow::Break`] to skip traversing into the
/// visited entity's children.
#[allow(unused_variables)]
pub trait CardVisitor {
	/// Called for entities that don't match any specific node type.
	fn visit_entity(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Block-level --

	/// Called for [`Heading`] entities.
	fn visit_heading(
		&mut self,
		entity: Entity,
		heading: &Heading,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`Paragraph`] entities.
	fn visit_paragraph(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`BlockQuote`] entities.
	fn visit_block_quote(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`CodeBlock`] entities.
	fn visit_code_block(
		&mut self,
		entity: Entity,
		code_block: &CodeBlock,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`ListMarker`] entities.
	fn visit_list(
		&mut self,
		entity: Entity,
		list_marker: &ListMarker,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`ListItem`] entities.
	fn visit_list_item(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`Table`] entities.
	fn visit_table(
		&mut self,
		entity: Entity,
		table: &Table,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`TableHead`] entities.
	fn visit_table_head(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`TableRow`] entities.
	fn visit_table_row(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`TableCell`] entities.
	fn visit_table_cell(&mut self, entity: Entity) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`ThematicBreak`] entities.
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

	// -- Inline --

	/// Called for [`TextNode`] entities.
	fn visit_text(
		&mut self,
		entity: Entity,
		text: &TextNode,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`Link`] entities.
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
}
