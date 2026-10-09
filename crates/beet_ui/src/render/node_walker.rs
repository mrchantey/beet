use crate::prelude::*;
use beet_core::prelude::*;

/// The components a [`NodeVisitor`] reads off one node.
pub type NodeView<'a> = (
	Option<&'a Doctype>,
	Option<&'a Comment>,
	Option<&'a Element>,
	Option<&'a Value>,
	Option<&'a Expression>,
	Option<&'a CData>,
	Option<&'a ProcessingInstruction>,
);

/// Walks a rendered tree depth-first through [`RenderTreeQuery`], so every
/// [`Portal`] renders the tree it transcludes in place, calling a
/// [`NodeVisitor`] on each node.
#[derive(SystemParam)]
pub struct NodeWalker<'w, 's> {
	elements: ElementQuery<'w, 's>,
	nodes: Query<'w, 's, NodeView<'static>>,
	tree: RenderTreeQuery<'w, 's>,
}

/// Where a [`NodeVisitor`] is in the walk.
pub struct VisitContext {
	/// The element/value node currently being visited.
	pub entity: Entity,
	/// Current depth in the tree, starting at 0 for the root.
	pub depth: usize,
}

impl NodeWalker<'_, '_> {
	/// Walk the tree at `entity`, a holder walking the tree it transcludes.
	pub fn walk(&self, visitor: &mut impl NodeVisitor, entity: Entity) {
		let cx = VisitContext {
			entity: self.tree.resolve(entity),
			depth: 0,
		};
		self.walk_entity(visitor, cx);
	}

	fn walk_entity(&self, visitor: &mut impl NodeVisitor, cx: VisitContext) {
		let Ok(node) = self.nodes.get(cx.entity) else {
			return;
		};

		if visitor.skip_node(&cx, &node) {
			return;
		}

		let (doctype, comment, element, value, expression, cdata, instruction) =
			node;

		// 1. Doctype
		if let Some(doctype) = doctype {
			visitor.visit_doctype(&cx, doctype);
		}
		// 2. Comment, processing instruction and CDATA, the other leaves
		if let Some(comment) = comment {
			visitor.visit_comment(&cx, comment);
		}
		if let Some(instruction) = instruction {
			visitor.visit_processing_instruction(&cx, instruction);
		}
		if let Some(cdata) = cdata {
			visitor.visit_cdata(&cx, cdata);
		}
		// 3. Element
		if let Ok(view) = self.elements.get(cx.entity) {
			visitor.visit_element(&cx, view);
		}
		// 4. Value
		// a text node: an entity whose Value is its content. An element's own
		// Value is binding state, not markup content, reaching the visitor only
		// on a form control, as the [`ElementView::value`] its `visit_element`
		// received (a null there is nothing typed rather than the word "null").
		// A bound text node still reads its null as the value it is, since a
		// view is total where an editor is empty.
		if let Some(value) = value
			&& element.is_none()
		{
			visitor.visit_value(&cx, value);
		}
		// 5. Expression
		if let Some(expression) = expression {
			visitor.visit_expression(&cx, expression);
		}
		// 6. Children
		for child in self.tree.children(cx.entity) {
			let child_cx = VisitContext {
				entity: child,
				depth: cx.depth + 1,
			};
			self.walk_entity(visitor, child_cx);
		}

		// 7. Leave Element
		if let Some(element) = element {
			visitor.leave_element(&cx, element);
		}
	}
}

/// Form-control tags whose own [`Value`] is their displayed content (eg the
/// charcell editable textbox, a served `<input>`'s `value`). Every other
/// element treats a co-located [`Value`] as binding state, never rendered as
/// text.
pub(crate) const VALUE_ELEMENT_TAGS: &[&str] = &["input", "textarea", "select"];

/// Whether a tag displays its own [`Value`], ie [`VALUE_ELEMENT_TAGS`].
pub(crate) fn is_value_element(tag: &str) -> bool {
	VALUE_ELEMENT_TAGS.contains(&tag)
}

/// Visits each node of a [`NodeWalker`] walk, in document order.
pub trait NodeVisitor {
	/// Return `true` to skip visiting this node and all its children.
	/// By default skips every tag carrying no text, ie `head, style, svg, ..`
	/// (see [`RenderTreeQuery::is_textless`]).
	fn skip_node(
		&mut self,
		_cx: &VisitContext,
		(_, _, element, ..): &NodeView,
	) -> bool {
		element
			.is_some_and(|element| RenderTreeQuery::is_textless(element.tag()))
	}

	fn visit_doctype(&mut self, _cx: &VisitContext, _doctype: &Doctype) {}
	fn visit_comment(&mut self, _cx: &VisitContext, _comment: &Comment) {}
	/// A processing instruction, ie `<?xml version="1.0"?>`: markup
	/// punctuation, which no reader sees.
	fn visit_processing_instruction(
		&mut self,
		_cx: &VisitContext,
		_instruction: &ProcessingInstruction,
	) {
	}
	/// A CDATA section, which a reader reads as the text it holds.
	fn visit_cdata(&mut self, cx: &VisitContext, cdata: &CData) {
		self.visit_value(cx, &Value::Str(cdata.0.as_str().into()));
	}
	fn visit_element(&mut self, _cx: &VisitContext, _view: ElementView) {}
	fn leave_element(&mut self, _cx: &VisitContext, _element: &Element) {}
	fn visit_value(&mut self, _cx: &VisitContext, _value: &Value) {}
	fn visit_expression(
		&mut self,
		_cx: &VisitContext,
		_expression: &Expression,
	) {
	}
}
