use crate::prelude::*;
use beet_core::prelude::*;


#[derive(SystemParam)]
pub struct NodeWalker<'w, 's> {
	// Core node identification
	nodes: Query<
		'w,
		's,
		(
			Option<&'static Comment>,
			Option<&'static Element>,
			Option<&'static Children>,
			Option<&'static Value>,
		),
	>,
	attributes: AttributeQuery<'w, 's>,
}


pub struct VisitContext {
	/// The entity from which the walker began
	start: Entity,
	/// The element/value node currently being visited
	entity: Entity,
	/// Whether this entity or an ancestor was
	/// marked as a comment
	comment: bool,
}

impl NodeWalker<'_, '_> {
	pub fn walk(&self, visitor: &mut impl NodeVisitor, entity: Entity) {
		let cx = VisitContext {
			start: entity,
			entity,
			// this may be updated by walk_entity
			comment: false,
		};
		self.walk_entity(visitor, cx);
	}

	fn walk_entity(
		&self,
		visitor: &mut impl NodeVisitor,
		mut cx: VisitContext,
	) {
		let Ok((comment, element, children, value)) = self.nodes.get(cx.entity)
		else {
			return;
		};
		// 1. Comment check
		if comment.is_some() {
			// may already be true
			cx.comment = true;
		}

		// 2. Walk Element
		if let Some(element) = element {
			let attrs = self.attributes.all(cx.entity);
			visitor.visit_element(&cx, element, attrs);
		}
		// 4. Walk Value
		if let Some(value) = value {
			visitor.visit_value(&cx, value);
		}
		// 5. Walk Children
		if let Some(children) = children {
			for child in children {
				let child_cx = VisitContext {
					start: cx.start,
					entity: *child,
					comment: cx.comment,
				};
				self.walk_entity(visitor, child_cx);
			}
		}

		// 6. Leave Element
		if let Some(element) = element {
			visitor.leave_element(&cx, element);
		}
	}
}


pub trait NodeVisitor {
	fn visit_element(
		&mut self,
		_cx: &VisitContext,
		_element: &Element,
		_attrs: Vec<(Entity, &Attribute, &Value)>,
	);
	fn leave_element(&mut self, _cx: &VisitContext, _element: &Element);
	fn visit_value(&mut self, _cx: &VisitContext, _value: &Value);
}


pub struct PlainTextRenderer {
	did_newline: bool,
	buffer: String,
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
			self.buffer.push_str("\n");
			self.did_newline = true;
		}
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		self.buffer.push_str(&value.to_string());
		self.did_newline = false;
	}
}
