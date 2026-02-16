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
//! # Visit Context
//!
//! The walker maintains a [`VisitContext`] that tracks shared
//! traversal state: the inline style stack, code block flag, list
//! nesting, and heading level. Every visitor method receives a
//! `&VisitContext` so renderers can query this state without
//! duplicating it.
//!
//! # Inline Style Stack
//!
//! When the walker enters an inline container (an entity with inline
//! marker components like [`Important`] but no [`TextNode`]), it
//! pushes the container's style onto the context's style stack. When
//! leaving, it pops. [`VisitContext::effective_style`] merges all
//! stack entries to produce the current inline formatting.
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
//!         ctx: &VisitContext,
//!         _entity: Entity,
//!         text: &TextNode,
//!     ) -> ControlFlow<()> {
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
/// Uses [`CardQuery`] for boundary detection and individual component
/// queries for node data. Dispatches on [`Node::kind`] rather than
/// per-component `contains()` checks.
#[derive(SystemParam)]
pub struct CardWalker<'w, 's> {
	card_query: CardQuery<'w, 's>,
	children_query: Query<'w, 's, &'static Children>,
	// Core node identification
	nodes: Query<'w, 's, &'static Node>,
	// Block-level data queries
	headings: Query<'w, 's, &'static Heading>,
	code_blocks: Query<'w, 's, &'static CodeBlock>,
	list_markers: Query<'w, 's, &'static ListMarker>,
	tables: Query<'w, 's, &'static Table>,
	images: Query<'w, 's, &'static Image>,
	footnote_defs: Query<'w, 's, &'static FootnoteDefinition>,
	html_blocks: Query<'w, 's, &'static HtmlBlock>,
	// Inline data queries
	text_nodes: Query<'w, 's, &'static TextNode>,
	links: Query<'w, 's, &'static Link>,
	footnote_refs: Query<'w, 's, &'static FootnoteRef>,
	html_inlines: Query<'w, 's, &'static HtmlInline>,
	task_checks: Query<'w, 's, &'static TaskListCheck>,
}

impl CardWalker<'_, '_> {
	/// Walk a card's entity tree depth-first, calling the visitor
	/// for each entity. Starts from the card root of `entity`.
	pub fn walk_card<V: CardVisitor>(&self, visitor: &mut V, entity: Entity) {
		let root = self.card_query.card_root(entity);
		let mut ctx = VisitContext::default();
		self.walk_entity(visitor, &mut ctx, root, root);
	}

	/// Walk from a specific entity without resolving the card root.
	pub fn walk_from<V: CardVisitor>(&self, visitor: &mut V, entity: Entity) {
		let mut ctx = VisitContext::default();
		self.walk_entity(visitor, &mut ctx, entity, entity);
	}

	/// Recursive depth-first walk. `root` is the walk origin, used
	/// to distinguish the starting entity from child card boundaries.
	fn walk_entity<V: CardVisitor>(
		&self,
		visitor: &mut V,
		ctx: &mut VisitContext,
		entity: Entity,
		root: Entity,
	) {
		let Ok(node) = self.nodes.get(entity) else {
			// Entity has no Node component — call visit_entity and
			// recurse into children (supports non-node structural
			// entities like the card root).
			let flow = visitor.visit_entity(ctx, entity);
			if flow.is_continue() {
				if let Ok(children) = self.children_query.get(entity) {
					for child in children.iter() {
						if child != root && self.card_query.is_card(child) {
							continue;
						}
						self.walk_entity(visitor, ctx, child, root);
					}
				}
			}
			return;
		};

		let kind = node.kind();

		// Check if this entity is an inline container. If so, push
		// its style onto the context stack so descendant TextNode
		// entities inherit it.
		let is_inline_container = kind.is_inline_container();
		if is_inline_container {
			let mut style: InlineStyle = kind
				.inline_modifier()
				.map(InlineStyle::from)
				.unwrap_or_default();
			// Link containers also carry link data
			if kind == NodeKind::Link {
				style.link = self.links.get(entity).ok().cloned();
			}
			ctx.push_style(style);
		}

		// Dispatch to the appropriate visitor method based on NodeKind
		let (flow, dispatch_kind) =
			self.dispatch_visit(visitor, ctx, entity, kind);

		// If the visitor returned Break, skip this entity's children
		if flow.is_break() {
			// Still call the leave method even when breaking
			self.dispatch_leave(visitor, ctx, entity, dispatch_kind);
			if is_inline_container {
				ctx.pop_style();
			}
			return;
		}

		// Recurse into children
		if let Ok(children) = self.children_query.get(entity) {
			for child in children.iter() {
				// Stop at card boundaries (unless it's the root itself)
				if child != root && self.card_query.is_card(child) {
					continue;
				}
				self.walk_entity(visitor, ctx, child, root);
			}
		}

		// Leave callback after children have been visited
		self.dispatch_leave(visitor, ctx, entity, dispatch_kind);

		// Pop style after leave so leave methods still see the style
		if is_inline_container {
			ctx.pop_style();
		}
	}

	/// Dispatch to the appropriate visitor method based on
	/// [`NodeKind`]. Returns the control flow AND internal dispatch
	/// kind so we can call the corresponding leave method later.
	///
	/// Also updates the [`VisitContext`] for structural elements
	/// (headings, code blocks, lists).
	fn dispatch_visit<V: CardVisitor>(
		&self,
		visitor: &mut V,
		ctx: &mut VisitContext,
		entity: Entity,
		kind: NodeKind,
	) -> (ControlFlow<()>, DispatchKind) {
		match kind {
			// -- Block-level --
			NodeKind::Heading => {
				if let Ok(heading) = self.headings.get(entity) {
					ctx.set_heading_level(heading.level());
					(
						visitor.visit_heading(ctx, entity, heading),
						DispatchKind::Heading,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}
			NodeKind::Paragraph => (
				visitor.visit_paragraph(ctx, entity),
				DispatchKind::Paragraph,
			),
			NodeKind::BlockQuote => (
				visitor.visit_block_quote(ctx, entity),
				DispatchKind::BlockQuote,
			),
			NodeKind::CodeBlock => {
				if let Ok(code_block) = self.code_blocks.get(entity) {
					ctx.in_code_block = true;
					(
						visitor.visit_code_block(ctx, entity, code_block),
						DispatchKind::CodeBlock,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}
			NodeKind::ListMarker => {
				if let Ok(list_marker) = self.list_markers.get(entity) {
					ctx.push_list(
						list_marker.ordered,
						list_marker.start.unwrap_or(1),
					);
					(
						visitor.visit_list(ctx, entity, list_marker),
						DispatchKind::List,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}
			NodeKind::ListItem => {
				(visitor.visit_list_item(ctx, entity), DispatchKind::ListItem)
			}
			NodeKind::Table => {
				if let Ok(table) = self.tables.get(entity) {
					(
						visitor.visit_table(ctx, entity, table),
						DispatchKind::Table,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}
			NodeKind::TableHead => (
				visitor.visit_table_head(ctx, entity),
				DispatchKind::TableHead,
			),
			NodeKind::TableRow => {
				(visitor.visit_table_row(ctx, entity), DispatchKind::TableRow)
			}
			NodeKind::TableCell => (
				visitor.visit_table_cell(ctx, entity),
				DispatchKind::TableCell,
			),
			NodeKind::ThematicBreak => (
				visitor.visit_thematic_break(ctx, entity),
				DispatchKind::ThematicBreak,
			),
			NodeKind::Image => {
				if let Ok(image) = self.images.get(entity) {
					(
						visitor.visit_image(ctx, entity, image),
						DispatchKind::Image,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}
			NodeKind::FootnoteDefinition => {
				if let Ok(footnote_def) = self.footnote_defs.get(entity) {
					(
						visitor.visit_footnote_definition(
							ctx,
							entity,
							footnote_def,
						),
						DispatchKind::FootnoteDefinition,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}
			NodeKind::MathDisplay => (
				visitor.visit_math_display(ctx, entity),
				DispatchKind::MathDisplay,
			),
			NodeKind::HtmlBlock => {
				if let Ok(html_block) = self.html_blocks.get(entity) {
					(
						visitor.visit_html_block(ctx, entity, html_block),
						DispatchKind::HtmlBlock,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}
			// Block-level kinds without visitor callbacks
			NodeKind::DefinitionList
			| NodeKind::DefinitionTitle
			| NodeKind::DefinitionDetails
			| NodeKind::MetadataBlock => {
				(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
			}

			// -- Form --
			NodeKind::Button => {
				let text = self.text_nodes.get(entity).ok();
				(
					visitor.visit_button(ctx, entity, text),
					DispatchKind::Button,
				)
			}
			NodeKind::TaskListCheck => {
				if let Ok(task_check) = self.task_checks.get(entity) {
					(
						visitor.visit_task_list_check(ctx, entity, task_check),
						DispatchKind::TaskListCheck,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}

			// -- Text --
			NodeKind::TextNode => {
				if let Ok(text) = self.text_nodes.get(entity) {
					(visitor.visit_text(ctx, entity, text), DispatchKind::Text)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}

			// -- Inline containers --
			// These push style in walk_entity; they have no
			// dedicated visitor call, just recurse into children.
			NodeKind::Important
			| NodeKind::Emphasize
			| NodeKind::Code
			| NodeKind::Quote
			| NodeKind::Strikethrough
			| NodeKind::Superscript
			| NodeKind::Subscript
			| NodeKind::MathInline => (ControlFlow::Continue(()), DispatchKind::Entity),

			// Link as inline container (has children)
			NodeKind::Link => {
				if let Ok(link) = self.links.get(entity) {
					(visitor.visit_link(ctx, entity, link), DispatchKind::Link)
				} else {
					(ControlFlow::Continue(()), DispatchKind::Entity)
				}
			}

			// -- Inline leaves --
			NodeKind::HardBreak => (
				visitor.visit_hard_break(ctx, entity),
				DispatchKind::HardBreak,
			),
			NodeKind::SoftBreak => (
				visitor.visit_soft_break(ctx, entity),
				DispatchKind::SoftBreak,
			),
			NodeKind::FootnoteRef => {
				if let Ok(footnote_ref) = self.footnote_refs.get(entity) {
					(
						visitor.visit_footnote_ref(ctx, entity, footnote_ref),
						DispatchKind::FootnoteRef,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}
			NodeKind::HtmlInline => {
				if let Ok(html_inline) = self.html_inlines.get(entity) {
					(
						visitor.visit_html_inline(ctx, entity, html_inline),
						DispatchKind::HtmlInline,
					)
				} else {
					(visitor.visit_entity(ctx, entity), DispatchKind::Entity)
				}
			}
		}
	}

	/// Dispatch the corresponding leave method for a previously
	/// visited dispatch kind. Also restores [`VisitContext`] state.
	fn dispatch_leave<V: CardVisitor>(
		&self,
		visitor: &mut V,
		ctx: &mut VisitContext,
		entity: Entity,
		kind: DispatchKind,
	) {
		match kind {
			DispatchKind::Heading => {
				visitor.leave_heading(ctx, entity);
				ctx.clear_heading_level();
			}
			DispatchKind::Paragraph => visitor.leave_paragraph(ctx, entity),
			DispatchKind::BlockQuote => visitor.leave_block_quote(ctx, entity),
			DispatchKind::CodeBlock => {
				visitor.leave_code_block(ctx, entity);
				ctx.in_code_block = false;
			}
			DispatchKind::List => {
				visitor.leave_list(ctx, entity);
				ctx.pop_list();
			}
			DispatchKind::ListItem => {
				visitor.leave_list_item(ctx, entity);
				if let Some(list) = ctx.current_list_mut() {
					list.current_index += 1;
				}
			}
			DispatchKind::Table => visitor.leave_table(ctx, entity),
			DispatchKind::TableHead => visitor.leave_table_head(ctx, entity),
			DispatchKind::TableRow => visitor.leave_table_row(ctx, entity),
			DispatchKind::TableCell => visitor.leave_table_cell(ctx, entity),
			// Types without meaningful leave semantics
			DispatchKind::ThematicBreak
			| DispatchKind::Image
			| DispatchKind::FootnoteDefinition
			| DispatchKind::MathDisplay
			| DispatchKind::HtmlBlock
			| DispatchKind::Button
			| DispatchKind::Text
			| DispatchKind::Link
			| DispatchKind::HardBreak
			| DispatchKind::SoftBreak
			| DispatchKind::FootnoteRef
			| DispatchKind::HtmlInline
			| DispatchKind::TaskListCheck
			| DispatchKind::Entity => {}
		}
	}
}

/// Internal tag used to pair a `visit_*` call with its `leave_*`
/// counterpart without storing the full entity data. Distinct from
/// [`NodeKind`] which is the public semantic type on [`Node`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchKind {
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
///
/// Every method receives a [`&VisitContext`](VisitContext) with shared
/// traversal state (style stack, code block flag, list nesting,
/// heading level). Renderers should query this context rather than
/// tracking duplicate state.
#[allow(unused_variables)]
pub trait CardVisitor {
	/// Called for entities that don't match any specific node type.
	fn visit_entity(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Block-level visit --

	/// Called when entering a [`Heading`] entity.
	fn visit_heading(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		heading: &Heading,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`Paragraph`] entity.
	fn visit_paragraph(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`BlockQuote`] entity.
	fn visit_block_quote(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`CodeBlock`] entity.
	fn visit_code_block(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		code_block: &CodeBlock,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`ListMarker`] entity.
	fn visit_list(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		list_marker: &ListMarker,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`ListItem`] entity.
	fn visit_list_item(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`Table`] entity.
	fn visit_table(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		table: &Table,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`TableHead`] entity.
	fn visit_table_head(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`TableRow`] entity.
	fn visit_table_row(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`TableCell`] entity.
	fn visit_table_cell(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`ThematicBreak`] entities (no children).
	fn visit_thematic_break(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`Image`] entities.
	fn visit_image(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		image: &Image,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`FootnoteDefinition`] entities.
	fn visit_footnote_definition(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		footnote_def: &FootnoteDefinition,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`MathDisplay`] entities.
	fn visit_math_display(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`HtmlBlock`] entities.
	fn visit_html_block(
		&mut self,
		ctx: &VisitContext,
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
		ctx: &VisitContext,
		entity: Entity,
		label: Option<&TextNode>,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Inline --

	/// Called for [`TextNode`] entities. Use
	/// [`VisitContext::effective_style`] to get the current inline
	/// formatting from the style stack.
	fn visit_text(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		text: &TextNode,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`Link`] entities that do NOT also have a
	/// [`TextNode`] (standalone link containers). When a link is on
	/// the same entity as text, it appears in
	/// [`InlineStyle::link`] via the style stack instead.
	fn visit_link(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		link: &Link,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`HardBreak`] entities.
	fn visit_hard_break(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`SoftBreak`] entities.
	fn visit_soft_break(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`FootnoteRef`] entities.
	fn visit_footnote_ref(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		footnote_ref: &FootnoteRef,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`HtmlInline`] entities.
	fn visit_html_inline(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		html_inline: &HtmlInline,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`TaskListCheck`] entities.
	fn visit_task_list_check(
		&mut self,
		ctx: &VisitContext,
		entity: Entity,
		task_check: &TaskListCheck,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Block-level leave --

	/// Called after leaving a [`Heading`] entity and all its children.
	fn leave_heading(&mut self, ctx: &VisitContext, entity: Entity) {}

	/// Called after leaving a [`Paragraph`] entity and all its children.
	fn leave_paragraph(&mut self, ctx: &VisitContext, entity: Entity) {}

	/// Called after leaving a [`BlockQuote`] entity and all its children.
	fn leave_block_quote(&mut self, ctx: &VisitContext, entity: Entity) {}

	/// Called after leaving a [`CodeBlock`] entity and all its children.
	fn leave_code_block(&mut self, ctx: &VisitContext, entity: Entity) {}

	/// Called after leaving a [`ListMarker`] entity and all its children.
	fn leave_list(&mut self, ctx: &VisitContext, entity: Entity) {}

	/// Called after leaving a [`ListItem`] entity and all its children.
	fn leave_list_item(&mut self, ctx: &VisitContext, entity: Entity) {}

	/// Called after leaving a [`Table`] entity and all its children.
	fn leave_table(&mut self, ctx: &VisitContext, entity: Entity) {}

	/// Called after leaving a [`TableHead`] entity and all its children.
	fn leave_table_head(&mut self, ctx: &VisitContext, entity: Entity) {}

	/// Called after leaving a [`TableRow`] entity and all its children.
	fn leave_table_row(&mut self, ctx: &VisitContext, entity: Entity) {}

	/// Called after leaving a [`TableCell`] entity and all its children.
	fn leave_table_cell(&mut self, ctx: &VisitContext, entity: Entity) {}
}


#[cfg(test)]
mod test {
	use super::*;

	struct EntityCounter(usize);

	impl CardVisitor for EntityCounter {
		fn visit_entity(
			&mut self,
			_ctx: &VisitContext,
			_entity: Entity,
		) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
		fn visit_heading(
			&mut self,
			_ctx: &VisitContext,
			_entity: Entity,
			_heading: &Heading,
		) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
		fn visit_paragraph(
			&mut self,
			_ctx: &VisitContext,
			_entity: Entity,
		) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
		fn visit_text(
			&mut self,
			_ctx: &VisitContext,
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
			_ctx: &VisitContext,
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
				_ctx: &VisitContext,
				_entity: Entity,
				_heading: &Heading,
			) -> ControlFlow<()> {
				ControlFlow::Break(())
			}
			fn visit_text(
				&mut self,
				_ctx: &VisitContext,
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

	#[test]
	fn inline_style_from_context() {
		// Important as a CONTAINER with TextNode child — style
		// comes from the context's style stack.
		let mut world = World::new();
		let card = world
			.spawn((Card, children![(Paragraph, children![(
				Important,
				children![TextNode::new("styled")],
			)])]))
			.id();

		struct StyleChecker(Vec<InlineStyle>);
		impl CardVisitor for StyleChecker {
			fn visit_text(
				&mut self,
				ctx: &VisitContext,
				_entity: Entity,
				_text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push(ctx.effective_style());
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
		styles[0].contains(InlineModifier::BOLD).xpect_true();
		styles[0].contains(InlineModifier::ITALIC).xpect_false();
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
				_ctx: &VisitContext,
				_entity: Entity,
				heading: &Heading,
			) -> ControlFlow<()> {
				self.0.push(format!("enter_h{}", heading.level()));
				ControlFlow::Continue(())
			}
			fn visit_text(
				&mut self,
				_ctx: &VisitContext,
				_entity: Entity,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push(format!("text:{}", text.as_str()));
				ControlFlow::Continue(())
			}
			fn leave_heading(&mut self, _ctx: &VisitContext, _entity: Entity) {
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
				_ctx: &VisitContext,
				_entity: Entity,
				_heading: &Heading,
			) -> ControlFlow<()> {
				self.0.push("enter_h".to_string());
				ControlFlow::Break(())
			}
			fn leave_heading(&mut self, _ctx: &VisitContext, _entity: Entity) {
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
		let style = InlineStyle::plain();
		style.is_plain().xpect_true();
	}

	#[test]
	fn non_plain_inline_style() {
		let style = InlineStyle::from(InlineModifier::BOLD);
		style.is_plain().xpect_false();
	}

	#[test]
	fn link_inline_style() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![(Paragraph, children![(
				Link::new("https://example.com"),
				children![TextNode::new("click")],
			)])]))
			.id();

		struct LinkChecker(Vec<InlineStyle>);
		impl CardVisitor for LinkChecker {
			fn visit_text(
				&mut self,
				ctx: &VisitContext,
				_entity: Entity,
				_text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push(ctx.effective_style());
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
				ctx: &VisitContext,
				_entity: Entity,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0
					.push((text.as_str().to_string(), ctx.effective_style()));
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
		collected[0].1.contains(InlineModifier::BOLD).xpect_false();
		// "bold" should inherit Important from parent container
		collected[1].0.xpect_eq("bold");
		collected[1].1.contains(InlineModifier::BOLD).xpect_true();
		// " end" should be plain again
		collected[2].0.xpect_eq(" end");
		collected[2].1.contains(InlineModifier::BOLD).xpect_false();
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
				ctx: &VisitContext,
				_entity: Entity,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0
					.push((text.as_str().to_string(), ctx.effective_style()));
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
		collected[0].1.contains(InlineModifier::BOLD).xpect_true();
		collected[0].1.contains(InlineModifier::ITALIC).xpect_true();
	}

	#[test]
	fn context_tracks_heading_level() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![
				(Heading2, children![TextNode::new("H2 text")]),
				(Paragraph, children![TextNode::new("Para text")]),
			]))
			.id();

		struct HeadingLevelChecker(Vec<(String, u8)>);
		impl CardVisitor for HeadingLevelChecker {
			fn visit_text(
				&mut self,
				ctx: &VisitContext,
				_entity: Entity,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0
					.push((text.as_str().to_string(), ctx.heading_level()));
				ControlFlow::Continue(())
			}
		}

		let collected = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut checker = HeadingLevelChecker(Vec::new());
					walker.walk_card(&mut checker, entity);
					checker.0
				},
				card,
			)
			.unwrap();

		collected.len().xpect_eq(2);
		collected[0].0.xpect_eq("H2 text");
		collected[0].1.xpect_eq(2);
		collected[1].0.xpect_eq("Para text");
		collected[1].1.xpect_eq(0);
	}

	#[test]
	fn context_tracks_code_block() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![
				(Paragraph, children![TextNode::new("before")]),
				(CodeBlock::plain(), children![TextNode::new("code")]),
				(Paragraph, children![TextNode::new("after")]),
			]))
			.id();

		struct CodeBlockChecker(Vec<(String, bool)>);
		impl CardVisitor for CodeBlockChecker {
			fn visit_text(
				&mut self,
				ctx: &VisitContext,
				_entity: Entity,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push((text.as_str().to_string(), ctx.in_code_block));
				ControlFlow::Continue(())
			}
		}

		let collected = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut checker = CodeBlockChecker(Vec::new());
					walker.walk_card(&mut checker, entity);
					checker.0
				},
				card,
			)
			.unwrap();

		collected.len().xpect_eq(3);
		collected[0].1.xpect_false(); // before
		collected[1].1.xpect_true(); // code
		collected[2].1.xpect_false(); // after
	}

	#[test]
	fn context_tracks_list_stack() {
		let mut world = World::new();
		let card = world
			.spawn((Card, children![(ListMarker::ordered(1), children![
				(ListItem, children![TextNode::new("first")]),
				(ListItem, children![TextNode::new("second")]),
			],)]))
			.id();

		struct ListChecker(Vec<(String, u64)>);
		impl CardVisitor for ListChecker {
			fn visit_text(
				&mut self,
				ctx: &VisitContext,
				_entity: Entity,
				text: &TextNode,
			) -> ControlFlow<()> {
				let num = ctx
					.current_list()
					.map(|list| list.current_number())
					.unwrap_or(0);
				self.0.push((text.as_str().to_string(), num));
				ControlFlow::Continue(())
			}
		}

		let collected = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut checker = ListChecker(Vec::new());
					walker.walk_card(&mut checker, entity);
					checker.0
				},
				card,
			)
			.unwrap();

		collected.len().xpect_eq(2);
		collected[0].0.xpect_eq("first");
		collected[0].1.xpect_eq(1);
		collected[1].0.xpect_eq("second");
		collected[1].1.xpect_eq(2);
	}
}
