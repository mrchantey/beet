use crate::prelude::*;
use beet_core::prelude::*;

pub type NodeView<'a> = (
	Option<&'a Doctype>,
	Option<&'a Comment>,
	Option<&'a Element>,
	Option<&'a Children>,
	Option<&'a Value>,
	Option<&'a Expression>,
);


#[derive(SystemParam)]
pub struct NodeWalker<'w, 's> {
	// Core node identification
	nodes: Query<'w, 's, NodeView<'static>>,
	attributes: AttributeQuery<'w, 's>,
}


pub struct VisitContext {
	/// The entity from which the walker began.
	pub start: Entity,
	/// The element/value node currently being visited.
	pub entity: Entity,
	/// Current depth in the tree, starting at 0 for the root.
	pub depth: usize,
}

/// Read-only view of an element and its attributes, provided to
/// [`NodeVisitor::visit_element`] for convenient attribute lookup.
pub struct ElementView<'a> {
	/// The element component.
	pub element: &'a Element,
	/// Attribute triples `(entity, key, value)` for this element.
	pub attributes: Vec<(Entity, &'a Attribute, &'a Value)>,
}

impl<'a> ElementView<'a> {
	/// Create a new view from an element reference and its attributes.
	pub fn new(
		element: &'a Element,
		attributes: Vec<(Entity, &'a Attribute, &'a Value)>,
	) -> Self {
		Self {
			element,
			attributes,
		}
	}

	/// The tag name of this element, ie `div`, `span`, `p`.
	pub fn name(&self) -> &str { self.element.name() }

	/// Look up the first attribute matching `key` and return its value.
	pub fn attribute(&self, key: &str) -> Option<&'a Value> {
		self.attributes
			.iter()
			.find(|(_, attr, _)| attr.as_str() == key)
			.map(|(_, _, val)| *val)
	}

	/// Look up the first attribute matching `key` and return its
	/// `(entity, value)` pair.
	pub fn attribute_with_entity(
		&self,
		key: &str,
	) -> Option<(Entity, &'a Value)> {
		self.attributes
			.iter()
			.find(|(_, attr, _)| attr.as_str() == key)
			.map(|(entity, _, val)| (*entity, *val))
	}

	/// Look up an attribute and convert its value to a [`String`].
	/// Returns an empty string when the attribute is absent.
	pub fn attribute_string(&self, key: &str) -> String {
		self.attribute(key)
			.map(|val| val.to_string())
			.unwrap_or_default()
	}

	/// Extract the `start` attribute as a `usize` for ordered lists.
	/// Defaults to `1` when absent or not numeric.
	pub fn ol_start(&self) -> usize {
		self.attribute("start")
			.and_then(|val| match val {
				Value::Uint(num) => Some(*num as usize),
				Value::Int(num) => Some(*num as usize),
				_ => None,
			})
			.unwrap_or(1)
	}
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
		let Ok(node) = self.nodes.get(cx.entity) else {
			return;
		};

		if visitor.skip_node(&node) {
			return;
		}

		let (doctype, comment, element, children, value, expression) = node;

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
			let view = ElementView::new(element, attrs);
			visitor.visit_element(&cx, &view);
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
	/// Return `true` to skip visiting this node and all its children.
	/// By default skips all non-visible html tags, ie `head, style, ..`
	fn skip_node(&mut self, (_, _, element, ..): &NodeView) -> bool {
		const HIDDEN_HTML_TAGS: &[&str] = &[
			"head", "script", "style", "template", "noscript", "iframe",
			"object", "embed",
		];
		if let Some(element) = element {
			HIDDEN_HTML_TAGS.iter().any(|tag| element.name() == *tag)
		} else {
			false
		}
	}

	fn visit_doctype(&mut self, _cx: &VisitContext, _doctype: &Doctype) {}
	fn visit_comment(&mut self, _cx: &VisitContext, _comment: &Comment) {}
	fn visit_element(&mut self, _cx: &VisitContext, _view: &ElementView) {}
	fn leave_element(&mut self, _cx: &VisitContext, _element: &Element) {}
	fn visit_value(&mut self, _cx: &VisitContext, _value: &Value) {}
	fn visit_expression(
		&mut self,
		_cx: &VisitContext,
		_expression: &Expression,
	) {
	}
}
