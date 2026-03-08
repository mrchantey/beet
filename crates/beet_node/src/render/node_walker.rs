use crate::prelude::*;
use beet_core::prelude::*;


#[derive(SystemParam)]
pub struct NodeWalker<'w, 's> {
	// Core node identification
	nodes: Query<
		'w,
		's,
		(
			Option<&'static Doctype>,
			Option<&'static Comment>,
			Option<&'static Element>,
			Option<&'static Children>,
			Option<&'static Value>,
			Option<&'static Expression>,
		),
	>,
	attributes: AttributeQuery<'w, 's>,
}


pub struct VisitContext {
	/// The entity from which the walker began
	pub start: Entity,
	/// The element/value node currently being visited
	pub entity: Entity,
	/// Current depth in the tree, starting at 0 for the root
	pub depth: usize,
}

impl NodeWalker<'_, '_> {
	pub fn walk(&self, visitor: &mut impl NodeVisitor, entity: Entity) {
		let cx = VisitContext {
			start: entity,
			entity,
			depth: 0,
		};
		self.walk_entity(visitor, cx);
	}

	fn walk_entity(&self, visitor: &mut impl NodeVisitor, cx: VisitContext) {
		let Ok((doctype, comment, element, children, value, expression)) =
			self.nodes.get(cx.entity)
		else {
			return;
		};

		// 1. Doctype
		if let Some(doctype) = doctype {
			visitor.visit_doctype(&cx, doctype);
		}
		// 2. Comment
		if let Some(comment) = comment {
			visitor.visit_comment(&cx, comment);
		}
		// 3. Element
		if let Some(element) = element {
			let attrs = self.attributes.all(cx.entity);
			visitor.visit_element(&cx, element, attrs);
		}
		// 4. Value
		if let Some(value) = value {
			visitor.visit_value(&cx, value);
		}
		// 5. Expression
		if let Some(expression) = expression {
			visitor.visit_expression(&cx, expression);
		}
		// 6. Children
		if let Some(children) = children {
			for child in children {
				let child_cx = VisitContext {
					start: cx.start,
					entity: *child,
					depth: cx.depth + 1,
				};
				self.walk_entity(visitor, child_cx);
			}
		}

		// 7. Leave Element
		if let Some(element) = element {
			visitor.leave_element(&cx, element);
		}
	}
}


pub trait NodeVisitor {
	fn visit_doctype(&mut self, _cx: &VisitContext, _doctype: &Doctype) {}
	fn visit_comment(&mut self, _cx: &VisitContext, _comment: &Comment) {}
	fn visit_element(
		&mut self,
		_cx: &VisitContext,
		_element: &Element,
		_attrs: Vec<(Entity, &Attribute, &Value)>,
	) {
	}
	fn leave_element(&mut self, _cx: &VisitContext, _element: &Element) {}
	fn visit_value(&mut self, _cx: &VisitContext, _value: &Value) {}
	fn visit_expression(
		&mut self,
		_cx: &VisitContext,
		_expression: &Expression,
	) {
	}
}


pub struct PlainTextRenderer {
	did_newline: bool,
	buffer: String,
}

impl PlainTextRenderer {
	pub fn new() -> Self {
		Self {
			did_newline: true,
			buffer: String::new(),
		}
	}

	/// Consume the renderer and return the accumulated text.
	pub fn into_string(self) -> String { self.buffer }
}

impl Default for PlainTextRenderer {
	fn default() -> Self { Self::new() }
}

impl NodeVisitor for PlainTextRenderer {
	fn visit_element(
		&mut self,
		_cx: &VisitContext,
		_element: &Element,
		_attrs: Vec<(Entity, &Attribute, &Value)>,
	) {
		// plaintext, ignore elements
	}

	fn leave_element(&mut self, _cx: &VisitContext, _element: &Element) {
		// add a newline after every element, except if we just added one
		if !self.did_newline {
			self.buffer.push('\n');
			self.did_newline = true;
		}
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		self.buffer.push_str(&value.to_string());
		self.did_newline = false;
	}
}
