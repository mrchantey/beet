//! Depth-first card traversal via the visitor pattern.
//!
//! [`CardWalker`] is a [`SystemParam`] that walks a card's entity
//! tree in depth-first order, dispatching to a [`CardVisitor`] for
//! each entity. The walker stops at child [`CardTool`] boundaries so
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
//! The walker maintains a [`VisitContext`] that tracks the current
//! entity, the inline style stack, code block flag, list nesting,
//! and heading level. Every visitor method receives a `&VisitContext`
//! so renderers can query this state without duplicating it. The
//! current entity is always available via
//! [`VisitContext::entity()`].
//!
//! # Inline Style Stack
//!
//! When the walker enters an inline container (an entity with inline
//! marker components like [`Important`] but no [`TextNode`]), it
//! pushes the container's modifier onto the context's style stack.
//! When leaving, it pops. [`VisitContext::effective_style`] merges
//! all stack entries to produce the current inline formatting.
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
//!         cx: &VisitContext,
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
/// queries for node data. Dispatches on [`Node`] variants rather
/// than per-component `contains()` checks.
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
	// Form data queries
	buttons: Query<'w, 's, &'static Button>,
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
		let mut cx = VisitContext::new(root);
		self.walk_entity(visitor, &mut cx, root, root);
	}

	/// Walk from a specific entity without resolving the card root.
	pub fn walk_from<V: CardVisitor>(&self, visitor: &mut V, entity: Entity) {
		let mut cx = VisitContext::new(entity);
		self.walk_entity(visitor, &mut cx, entity, entity);
	}

	/// Recurse into children of `entity`, skipping child card
	/// boundaries (unless they are the walk root itself).
	fn recurse_children<V: CardVisitor>(
		&self,
		visitor: &mut V,
		cx: &mut VisitContext,
		entity: Entity,
		root: Entity,
	) {
		if let Ok(children) = self.children_query.get(entity) {
			for child in children.iter() {
				if child != root && self.card_query.is_card(child) {
					continue;
				}
				self.walk_entity(visitor, cx, child, root);
			}
		}
	}

	/// Recursive depth-first walk. `root` is the walk origin, used
	/// to distinguish the starting entity from child card boundaries.
	fn walk_entity<V: CardVisitor>(
		&self,
		visitor: &mut V,
		cx: &mut VisitContext,
		entity: Entity,
		root: Entity,
	) {
		let Ok(node) = self.nodes.get(entity) else {
			// Entity has no Node component — call visit_entity and
			// recurse into children (supports non-node structural
			// entities like the card root).
			cx.set_entity(entity);
			let flow = visitor.visit_entity(cx);
			if flow.is_continue() {
				self.recurse_children(visitor, cx, entity, root);
			}
			return;
		};

		let kind = *node;

		// Check if this entity is an inline container. If so, push
		// its style onto the context stack so descendant TextNode
		// entities inherit it.
		let is_inline_container = kind.is_inline_container();
		if is_inline_container {
			let style = kind.inline_style().unwrap_or_default();
			cx.push_style(style);
		}

		cx.set_entity(entity);

		// Dispatch visit, recurse children, then leave — all within
		// a single match so visit/leave correspondence is explicit.
		match kind {
			// ---- Block-level ----
			Node::Heading => {
				if let Ok(heading) = self.headings.get(entity) {
					cx.set_heading_level(heading.level());
					let flow = visitor.visit_heading(cx, heading);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
					cx.set_entity(entity);
					visitor.leave_heading(cx, heading);
					cx.clear_heading_level();
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			Node::Paragraph => {
				let flow = visitor.visit_paragraph(cx);
				if flow.is_continue() {
					self.recurse_children(visitor, cx, entity, root);
				}
				cx.set_entity(entity);
				visitor.leave_paragraph(cx);
			}

			Node::BlockQuote => {
				let flow = visitor.visit_block_quote(cx);
				if flow.is_continue() {
					self.recurse_children(visitor, cx, entity, root);
				}
				cx.set_entity(entity);
				visitor.leave_block_quote(cx);
			}

			Node::CodeBlock => {
				if let Ok(code_block) = self.code_blocks.get(entity) {
					cx.in_code_block = true;
					let flow = visitor.visit_code_block(cx, code_block);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
					cx.set_entity(entity);
					visitor.leave_code_block(cx, code_block);
					cx.in_code_block = false;
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			Node::ListMarker => {
				if let Ok(list_marker) = self.list_markers.get(entity) {
					cx.push_list(
						list_marker.ordered,
						list_marker.start.unwrap_or(1),
					);
					let flow = visitor.visit_list(cx, list_marker);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
					cx.set_entity(entity);
					visitor.leave_list(cx, list_marker);
					cx.pop_list();
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			Node::ListItem => {
				let flow = visitor.visit_list_item(cx);
				if flow.is_continue() {
					self.recurse_children(visitor, cx, entity, root);
				}
				cx.set_entity(entity);
				visitor.leave_list_item(cx);
				if let Some(list) = cx.current_list_mut() {
					list.current_index += 1;
				}
			}

			Node::Table => {
				if let Ok(table) = self.tables.get(entity) {
					let flow = visitor.visit_table(cx, table);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
					cx.set_entity(entity);
					visitor.leave_table(cx, table);
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			Node::TableHead => {
				let flow = visitor.visit_table_head(cx);
				if flow.is_continue() {
					self.recurse_children(visitor, cx, entity, root);
				}
				cx.set_entity(entity);
				visitor.leave_table_head(cx);
			}

			Node::TableRow => {
				let flow = visitor.visit_table_row(cx);
				if flow.is_continue() {
					self.recurse_children(visitor, cx, entity, root);
				}
				cx.set_entity(entity);
				visitor.leave_table_row(cx);
			}

			Node::TableCell => {
				let flow = visitor.visit_table_cell(cx);
				if flow.is_continue() {
					self.recurse_children(visitor, cx, entity, root);
				}
				cx.set_entity(entity);
				visitor.leave_table_cell(cx);
			}

			Node::ThematicBreak => {
				let _flow = visitor.visit_thematic_break(cx);
				// ThematicBreak is a leaf — no children to recurse
			}

			Node::Image => {
				if let Ok(image) = self.images.get(entity) {
					let flow = visitor.visit_image(cx, image);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
					cx.set_entity(entity);
					visitor.leave_image(cx, image);
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			Node::FootnoteDefinition => {
				if let Ok(footnote_def) = self.footnote_defs.get(entity) {
					let flow =
						visitor.visit_footnote_definition(cx, footnote_def);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			Node::MathDisplay => {
				let flow = visitor.visit_math_display(cx);
				if flow.is_continue() {
					self.recurse_children(visitor, cx, entity, root);
				}
			}

			Node::HtmlBlock => {
				if let Ok(html_block) = self.html_blocks.get(entity) {
					let flow = visitor.visit_html_block(cx, html_block);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
					cx.set_entity(entity);
					visitor.leave_html_block(cx, html_block);
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			// Block-level nodes without dedicated visitor callbacks
			Node::DefinitionList
			| Node::DefinitionTitle
			| Node::DefinitionDetails
			| Node::MetadataBlock => {
				let flow = visitor.visit_entity(cx);
				if flow.is_continue() {
					self.recurse_children(visitor, cx, entity, root);
				}
			}

			// ---- Form ----
			Node::Button => {
				if let Ok(button) = self.buttons.get(entity) {
					let flow = visitor.visit_button(cx, button);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
					cx.set_entity(entity);
					visitor.leave_button(cx, button);
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			Node::TaskListCheck => {
				if let Ok(task_check) = self.task_checks.get(entity) {
					let flow = visitor.visit_task_list_check(cx, task_check);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			// ---- Text ----
			Node::TextNode => {
				if let Ok(text) = self.text_nodes.get(entity) {
					let _flow = visitor.visit_text(cx, text);
				} else {
					let flow = visitor.visit_entity(cx);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
				}
			}

			// ---- Inline containers ----
			// These push style in the block above; they have no
			// dedicated visitor call, just recurse into children.
			Node::Important
			| Node::Emphasize
			| Node::Code
			| Node::Quote
			| Node::Strikethrough
			| Node::Superscript
			| Node::Subscript
			| Node::MathInline => {
				self.recurse_children(visitor, cx, entity, root);
			}

			// Link as inline container (has children and a
			// dedicated visit/leave pair)
			Node::Link => {
				if let Ok(link) = self.links.get(entity) {
					let flow = visitor.visit_link(cx, link);
					if flow.is_continue() {
						self.recurse_children(visitor, cx, entity, root);
					}
					cx.set_entity(entity);
					visitor.leave_link(cx, link);
				} else {
					self.recurse_children(visitor, cx, entity, root);
				}
			}

			// ---- Inline leaves ----
			Node::HardBreak => {
				let _flow = visitor.visit_hard_break(cx);
			}
			Node::SoftBreak => {
				let _flow = visitor.visit_soft_break(cx);
			}
			Node::FootnoteRef => {
				if let Ok(footnote_ref) = self.footnote_refs.get(entity) {
					let _flow = visitor.visit_footnote_ref(cx, footnote_ref);
				} else {
					let _flow = visitor.visit_entity(cx);
				}
			}
			Node::HtmlInline => {
				if let Ok(html_inline) = self.html_inlines.get(entity) {
					let _flow = visitor.visit_html_inline(cx, html_inline);
				} else {
					let _flow = visitor.visit_entity(cx);
				}
			}
		}

		// Pop style after leave so leave methods still see the style
		if is_inline_container {
			cx.pop_style();
		}
	}
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
/// traversal state (current entity, style stack, code block flag,
/// list nesting, heading level). The current entity is available via
/// [`VisitContext::entity()`]. Renderers should query this context
/// rather than tracking duplicate state.
#[allow(unused_variables)]
pub trait CardVisitor {
	/// Called for entities that don't match any specific node type.
	fn visit_entity(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Block-level visit --

	/// Called when entering a [`Heading`] entity.
	fn visit_heading(
		&mut self,
		cx: &VisitContext,
		heading: &Heading,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`Paragraph`] entity.
	fn visit_paragraph(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`BlockQuote`] entity.
	fn visit_block_quote(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`CodeBlock`] entity.
	fn visit_code_block(
		&mut self,
		cx: &VisitContext,
		code_block: &CodeBlock,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`ListMarker`] entity.
	fn visit_list(
		&mut self,
		cx: &VisitContext,
		list_marker: &ListMarker,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`ListItem`] entity.
	fn visit_list_item(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`Table`] entity.
	fn visit_table(
		&mut self,
		cx: &VisitContext,
		table: &Table,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`TableHead`] entity.
	fn visit_table_head(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`TableRow`] entity.
	fn visit_table_row(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`TableCell`] entity.
	fn visit_table_cell(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`ThematicBreak`] entities (no children).
	fn visit_thematic_break(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering an [`Image`] entity.
	fn visit_image(
		&mut self,
		cx: &VisitContext,
		image: &Image,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`FootnoteDefinition`] entities.
	fn visit_footnote_definition(
		&mut self,
		cx: &VisitContext,
		footnote_def: &FootnoteDefinition,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`MathDisplay`] entities.
	fn visit_math_display(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering an [`HtmlBlock`] entity.
	fn visit_html_block(
		&mut self,
		cx: &VisitContext,
		html_block: &HtmlBlock,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Form elements --

	/// Called when entering a [`Button`] entity. The button label
	/// is stored in [`TextNode`] children, visited after this.
	fn visit_button(
		&mut self,
		cx: &VisitContext,
		button: &Button,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Inline --

	/// Called for [`TextNode`] entities. Use
	/// [`VisitContext::effective_style`] to get the current inline
	/// formatting from the style stack.
	fn visit_text(
		&mut self,
		cx: &VisitContext,
		text: &TextNode,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called when entering a [`Link`] container. The link's style
	/// modifier (`LINK`) is already on the style stack via the
	/// inline container mechanism.
	fn visit_link(
		&mut self,
		cx: &VisitContext,
		link: &Link,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`HardBreak`] entities.
	fn visit_hard_break(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`SoftBreak`] entities.
	fn visit_soft_break(&mut self, cx: &VisitContext) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`FootnoteRef`] entities.
	fn visit_footnote_ref(
		&mut self,
		cx: &VisitContext,
		footnote_ref: &FootnoteRef,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`HtmlInline`] entities.
	fn visit_html_inline(
		&mut self,
		cx: &VisitContext,
		html_inline: &HtmlInline,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	/// Called for [`TaskListCheck`] entities.
	fn visit_task_list_check(
		&mut self,
		cx: &VisitContext,
		task_check: &TaskListCheck,
	) -> ControlFlow<()> {
		ControlFlow::Continue(())
	}

	// -- Block-level leave --

	/// Called after leaving a [`Heading`] entity and all its children.
	fn leave_heading(&mut self, cx: &VisitContext, heading: &Heading) {}

	/// Called after leaving a [`Paragraph`] entity and all its children.
	fn leave_paragraph(&mut self, cx: &VisitContext) {}

	/// Called after leaving a [`BlockQuote`] entity and all its children.
	fn leave_block_quote(&mut self, cx: &VisitContext) {}

	/// Called after leaving a [`CodeBlock`] entity and all its children.
	fn leave_code_block(&mut self, cx: &VisitContext, code_block: &CodeBlock) {}

	/// Called after leaving a [`ListMarker`] entity and all its children.
	fn leave_list(&mut self, cx: &VisitContext, list_marker: &ListMarker) {}

	/// Called after leaving a [`ListItem`] entity and all its children.
	fn leave_list_item(&mut self, cx: &VisitContext) {}

	/// Called after leaving a [`Table`] entity and all its children.
	fn leave_table(&mut self, cx: &VisitContext, table: &Table) {}

	/// Called after leaving a [`TableHead`] entity and all its children.
	fn leave_table_head(&mut self, cx: &VisitContext) {}

	/// Called after leaving a [`TableRow`] entity and all its children.
	fn leave_table_row(&mut self, cx: &VisitContext) {}

	/// Called after leaving a [`TableCell`] entity and all its children.
	fn leave_table_cell(&mut self, cx: &VisitContext) {}

	/// Called after leaving a [`Link`] container and all its children.
	fn leave_link(&mut self, cx: &VisitContext, link: &Link) {}

	/// Called after leaving an [`Image`] entity and all its children.
	fn leave_image(&mut self, cx: &VisitContext, image: &Image) {}

	/// Called after leaving an [`HtmlBlock`] entity and all its children.
	fn leave_html_block(&mut self, cx: &VisitContext, html_block: &HtmlBlock) {}

	/// Called after leaving a [`Button`] entity and all its children.
	fn leave_button(&mut self, cx: &VisitContext, button: &Button) {}
}


#[cfg(test)]
mod test {
	use super::*;

	struct EntityCounter(usize);

	impl CardVisitor for EntityCounter {
		fn visit_entity(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
		fn visit_heading(
			&mut self,
			_cx: &VisitContext,
			_heading: &Heading,
		) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
		fn visit_paragraph(&mut self, _cx: &VisitContext) -> ControlFlow<()> {
			self.0 += 1;
			ControlFlow::Continue(())
		}
		fn visit_text(
			&mut self,
			_cx: &VisitContext,
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
			.spawn((CardTool, children![
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
			.spawn((CardTool, children![
				(Paragraph, children![TextNode::new("parent")]),
				(CardTool, children![(Paragraph, children![TextNode::new(
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
			_cx: &VisitContext,
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
			.spawn((CardTool, children![
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
			.spawn((CardTool, children![
				(Heading1, children![TextNode::new("Skipped")]),
				(Paragraph, children![TextNode::new("Not skipped")]),
			]))
			.id();

		struct BreakHeadingTextCollector(String);
		impl CardVisitor for BreakHeadingTextCollector {
			fn visit_heading(
				&mut self,
				_cx: &VisitContext,
				_heading: &Heading,
			) -> ControlFlow<()> {
				ControlFlow::Break(())
			}
			fn visit_text(
				&mut self,
				_cx: &VisitContext,
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
			.spawn((CardTool, children![(Paragraph, children![(
				Important,
				children![TextNode::new("styled")],
			)])]))
			.id();

		struct StyleChecker(Vec<InlineStyle>);
		impl CardVisitor for StyleChecker {
			fn visit_text(
				&mut self,
				cx: &VisitContext,
				_text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push(cx.effective_style());
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
		styles[0].contains(InlineStyle::BOLD).xpect_true();
		styles[0].contains(InlineStyle::ITALIC).xpect_false();
	}

	#[test]
	fn leave_methods_called() {
		let mut world = World::new();
		let card = world
			.spawn((CardTool, children![(Heading1, children![TextNode::new(
				"Title"
			)]),]))
			.id();

		struct LifecycleTracker(Vec<String>);
		impl CardVisitor for LifecycleTracker {
			fn visit_heading(
				&mut self,
				_cx: &VisitContext,
				heading: &Heading,
			) -> ControlFlow<()> {
				self.0.push(format!("enter_h{}", heading.level()));
				ControlFlow::Continue(())
			}
			fn visit_text(
				&mut self,
				_cx: &VisitContext,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push(format!("text:{}", text.as_str()));
				ControlFlow::Continue(())
			}
			fn leave_heading(
				&mut self,
				_cx: &VisitContext,
				_heading: &Heading,
			) {
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
			.spawn((CardTool, children![(Heading1, children![TextNode::new(
				"Skipped"
			)]),]))
			.id();

		struct BreakTracker(Vec<String>);
		impl CardVisitor for BreakTracker {
			fn visit_heading(
				&mut self,
				_cx: &VisitContext,
				_heading: &Heading,
			) -> ControlFlow<()> {
				self.0.push("enter_h".to_string());
				ControlFlow::Break(())
			}
			fn leave_heading(
				&mut self,
				_cx: &VisitContext,
				_heading: &Heading,
			) {
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
		let style = InlineStyle::BOLD;
		style.is_plain().xpect_false();
	}

	#[test]
	fn link_inline_style() {
		let mut world = World::new();
		let card = world
			.spawn((CardTool, children![(Paragraph, children![(
				Link::new("https://example.com"),
				children![TextNode::new("click")],
			)])]))
			.id();

		struct LinkChecker(Vec<InlineStyle>);
		impl CardVisitor for LinkChecker {
			fn visit_text(
				&mut self,
				cx: &VisitContext,
				_text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push(cx.effective_style());
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
		styles[0].contains(InlineStyle::LINK).xpect_true();
	}

	#[test]
	fn inherited_style_from_container() {
		// Mimics the markdown parser pattern where Important is a
		// container entity with TextNode children, rather than a
		// marker on the same entity as TextNode.
		let mut world = World::new();
		let card = world
			.spawn((CardTool, children![(Paragraph, children![
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
				cx: &VisitContext,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0
					.push((text.as_str().to_string(), cx.effective_style()));
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
		collected[0].1.contains(InlineStyle::BOLD).xpect_false();
		// "bold" should inherit Important from parent container
		collected[1].0.xpect_eq("bold");
		collected[1].1.contains(InlineStyle::BOLD).xpect_true();
		// " end" should be plain again
		collected[2].0.xpect_eq(" end");
		collected[2].1.contains(InlineStyle::BOLD).xpect_false();
	}

	#[test]
	fn nested_inherited_styles() {
		// Important container wrapping an Emphasize container
		let mut world = World::new();
		let card = world
			.spawn((CardTool, children![(Paragraph, children![(
				Important,
				children![(Emphasize, children![TextNode::new("bold italic")])],
			)])]))
			.id();

		struct StyleCollector(Vec<(String, InlineStyle)>);
		impl CardVisitor for StyleCollector {
			fn visit_text(
				&mut self,
				cx: &VisitContext,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0
					.push((text.as_str().to_string(), cx.effective_style()));
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
		collected[0].1.contains(InlineStyle::BOLD).xpect_true();
		collected[0].1.contains(InlineStyle::ITALIC).xpect_true();
	}

	#[test]
	fn context_tracks_heading_level() {
		let mut world = World::new();
		let card = world
			.spawn((CardTool, children![
				(Heading2, children![TextNode::new("H2 text")]),
				(Paragraph, children![TextNode::new("Para text")]),
			]))
			.id();

		struct HeadingLevelChecker(Vec<(String, u8)>);
		impl CardVisitor for HeadingLevelChecker {
			fn visit_text(
				&mut self,
				cx: &VisitContext,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push((text.as_str().to_string(), cx.heading_level()));
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
			.spawn((CardTool, children![
				(Paragraph, children![TextNode::new("before")]),
				(CodeBlock::plain(), children![TextNode::new("code")]),
				(Paragraph, children![TextNode::new("after")]),
			]))
			.id();

		struct CodeBlockChecker(Vec<(String, bool)>);
		impl CardVisitor for CodeBlockChecker {
			fn visit_text(
				&mut self,
				cx: &VisitContext,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push((text.as_str().to_string(), cx.in_code_block));
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
			.spawn((CardTool, children![(ListMarker::ordered(1), children![
				(ListItem, children![TextNode::new("first")]),
				(ListItem, children![TextNode::new("second")]),
			],)]))
			.id();

		struct ListChecker(Vec<(String, u64)>);
		impl CardVisitor for ListChecker {
			fn visit_text(
				&mut self,
				cx: &VisitContext,
				text: &TextNode,
			) -> ControlFlow<()> {
				let num = cx
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

	#[test]
	fn context_entity_tracks_current() {
		let mut world = World::new();
		let card = world
			.spawn((CardTool, children![(Paragraph, children![TextNode::new(
				"hello"
			)])]))
			.id();

		struct EntityTracker(Vec<Entity>);
		impl CardVisitor for EntityTracker {
			fn visit_text(
				&mut self,
				cx: &VisitContext,
				_text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push(cx.entity());
				ControlFlow::Continue(())
			}
		}

		let entities = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut tracker = EntityTracker(Vec::new());
					walker.walk_card(&mut tracker, entity);
					tracker.0
				},
				card,
			)
			.unwrap();

		entities.len().xpect_eq(1);
		// The tracked entity should have a TextNode
		world
			.entity(entities[0])
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("hello");
	}

	#[test]
	fn leave_link_called() {
		let mut world = World::new();
		let card = world
			.spawn((CardTool, children![(Paragraph, children![(
				Link::new("https://example.com"),
				children![TextNode::new("click")],
			)])]))
			.id();

		struct LinkTracker(Vec<String>);
		impl CardVisitor for LinkTracker {
			fn visit_link(
				&mut self,
				_cx: &VisitContext,
				link: &Link,
			) -> ControlFlow<()> {
				self.0.push(format!("enter:{}", link.href));
				ControlFlow::Continue(())
			}
			fn visit_text(
				&mut self,
				_cx: &VisitContext,
				text: &TextNode,
			) -> ControlFlow<()> {
				self.0.push(format!("text:{}", text.as_str()));
				ControlFlow::Continue(())
			}
			fn leave_link(&mut self, _cx: &VisitContext, _link: &Link) {
				self.0.push("leave_link".to_string());
			}
		}

		let events = world
			.run_system_once_with(
				|In(entity): In<Entity>, walker: CardWalker| {
					let mut tracker = LinkTracker(Vec::new());
					walker.walk_card(&mut tracker, entity);
					tracker.0
				},
				card,
			)
			.unwrap();

		events.xpect_eq(vec![
			"enter:https://example.com".to_string(),
			"text:click".to_string(),
			"leave_link".to_string(),
		]);
	}
}
