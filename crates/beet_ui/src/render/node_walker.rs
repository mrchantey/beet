use crate::prelude::*;
use beet_core::prelude::*;

pub type NodeView<'a> = (
	Option<&'a Doctype>,
	Option<&'a Comment>,
	Option<&'a Element>,
	Option<&'a Children>,
	Option<&'a Value>,
	Option<&'a Expression>,
	Option<&'a RenderRef>,
);


#[derive(SystemParam)]
pub struct NodeWalker<'w, 's> {
	elements: ElementQuery<'w, 's>,
	nodes: Query<'w, 's, NodeView<'static>>,
}


pub struct VisitContext {
	/// The entity from which the walker began.
	pub start: Entity,
	/// The element/value node currently being visited.
	pub entity: Entity,
	/// Current depth in the tree, starting at 0 for the root.
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
		let Ok(node) = self.nodes.get(cx.entity) else {
			return;
		};

		if visitor.skip_node(&node) {
			return;
		}

		let (
			doctype,
			comment,
			element,
			children,
			value,
			expression,
			render_ref,
		) = node;

		// A RenderRef holder is transparent: recurse directly into the
		// referenced entity, rendering it in place without touching this
		// entity's own components.
		if let Some(render_ref) = render_ref {
			let child_cx = VisitContext {
				start: cx.start,
				entity: **render_ref,
				depth: cx.depth,
			};
			self.walk_entity(visitor, child_cx);
			return;
		}

		// 1. Doctype
		if let Some(doctype) = doctype {
			visitor.visit_doctype(&cx, doctype);
		}
		// 2. Comment
		if let Some(comment) = comment {
			visitor.visit_comment(&cx, comment);
		}
		// 3. Element
		if let Ok(view) = self.elements.get(cx.entity) {
			visitor.visit_element(&cx, view);
		}
		// 4. Value
		// note: elements can also have a value
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

/// HTML tags that carry no visual content: document metadata, scripting, and
/// embedded resources. This is the single source of truth for "non-visual",
/// consumed in two places that must agree:
/// - the user-agent style layer ([`default_element_rules`]) maps these to
///   [`Display::None`] so the style-resolved visual renderer (charcell) omits
///   them via its `display: none` filter, and
/// - [`NodeVisitor::skip_node`] skips them for the markup/text renderers
///   (markdown, plaintext, tui) that walk the raw node tree without resolving
///   CSS, so `display: none` is unavailable to them.
///
/// A markup serializer ([`HtmlRenderer`]) is the exception: it overrides
/// `skip_node` to emit every tag, since `<head>`/`<style>`/`<script>` are valid,
/// non-visual-but-serialized HTML.
///
/// [`Display::None`]: crate::style::Display
/// [`default_element_rules`]: crate::style::default_element_rules
/// [`HtmlRenderer`]: crate::prelude::HtmlRenderer
pub const NON_VISUAL_TAGS: &[&str] = &[
	"head", "script", "style", "template", "noscript", "meta", "link", "title",
	"base", "iframe", "object", "embed",
];

/// Whether a tag carries no visual content, ie [`NON_VISUAL_TAGS`].
pub fn is_non_visual(tag: &str) -> bool { NON_VISUAL_TAGS.contains(&tag) }

pub trait NodeVisitor {
	/// Return `true` to skip visiting this node and all its children.
	/// By default skips all non-visual html tags, ie `head, style, ..`
	fn skip_node(&mut self, (_, _, element, ..): &NodeView) -> bool {
		element.is_some_and(|element| is_non_visual(element.tag()))
	}

	fn visit_doctype(&mut self, _cx: &VisitContext, _doctype: &Doctype) {}
	fn visit_comment(&mut self, _cx: &VisitContext, _comment: &Comment) {}
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
