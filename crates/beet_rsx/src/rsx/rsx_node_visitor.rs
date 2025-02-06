use crate::prelude::*;

/// Walking trees like rsx is deceptively difficult.
/// The visitor pattern handles the 'walking' and allows implementers
/// to focus on the 'visiting'.
/// implement options
///
/// Visiting fragments is intentionally not supported,
/// they are by definition transparent so depending on them
/// is considered an antipattern.
#[allow(unused_variables)]
pub trait RsxNodeVisitor {
	/// defaults to true, override for custom behavior
	fn walk_element_children(&self) -> bool { true }
	/// defaults to true, override for custom behavior
	fn walk_component_node(&self) -> bool { true }
	/// defaults to true, override for custom behavior
	fn walk_component_slot_children(&self) -> bool { true }
	/// begin walking the nodes
	fn walk(&mut self, node: &RsxNode) -> RsxContext {
		let mut cx = RsxContext::default();
		let mut queue = std::collections::VecDeque::new();
		queue.push_back(node);

		while let Some(current) = queue.pop_front() {
			let node_type = current.discriminant();
			match current {
				RsxNode::Doctype => self.visit_doctype(&cx),
				RsxNode::Comment(c) => self.visit_comment(&cx, c),
				RsxNode::Text(t) => self.visit_text(&cx, t),
				RsxNode::Block(b) => self.visit_block(&cx, b),
				RsxNode::Fragment(f) => {
					queue.extend(f);
				}
				RsxNode::Element(e) => {
					self.visit_element(&cx, e);
					for attr in &e.attributes {
						self.visit_attribute(&cx, attr);
					}
					if self.walk_element_children() {
						queue.extend(&e.children);
					}
				}
				RsxNode::Component(c) => {
					self.visit_component(&cx, c);
					if self.walk_component_node() {
						queue.push_back(&c.node);
					}
				}
			}
			cx.node_idx += 1;
		}
		cx
	}

	fn visit_doctype(&mut self, cx: &RsxContext) {}
	fn visit_comment(&mut self, cx: &RsxContext, comment: &str) {}
	fn visit_text(&mut self, cx: &RsxContext, text: &str) {}
	fn visit_block(&mut self, cx: &RsxContext, block: &RsxBlock) {}
	fn visit_component(&mut self, cx: &RsxContext, component: &RsxComponent) {}
	fn visit_element(&mut self, cx: &RsxContext, element: &RsxElement) {}
	fn visit_attribute(&mut self, cx: &RsxContext, attribute: &RsxAttribute) {}
}

impl RsxNode {
	/// Depth first traversal of the tree
	pub fn visit(&self, mut func: impl FnMut(&RsxNode)) {
		fn inner(node: &RsxNode, func: &mut impl FnMut(&RsxNode)) {
			func(node);
			for child in node.children() {
				inner(child, func);
			}
		}
		inner(self, &mut func);
	}
	/// Depth first mutable traversal of the tree
	pub fn visit_mut(&mut self, mut func: impl FnMut(&mut RsxNode)) {
		fn inner(node: &mut RsxNode, func: &mut impl FnMut(&mut RsxNode)) {
			func(node);
			for child in node.children_mut() {
				inner(child, func);
			}
		}
		inner(self, &mut func);
	}

	/// Depth first traversal of the tree, will not visit child components,
	/// this is useful for patterns like [ScopedStyle]
	pub fn visit_ignore_components(&self, mut func: impl FnMut(&RsxNode)) {
		fn inner(node: &RsxNode, func: &mut impl FnMut(&RsxNode)) {
			if let RsxNode::Component(_) = node {
				return;
			}
			func(node);
			for child in node.children() {
				inner(child, func);
			}
		}
		inner(self, &mut func);
	}
	/// Depth first mutable traversal of the tree, will not visit child components,
	/// this is useful for patterns like [ScopedStyle]
	pub fn visit_ignore_components_mut(
		&mut self,
		mut func: impl FnMut(&mut RsxNode),
	) {
		fn inner(node: &mut RsxNode, func: &mut impl FnMut(&mut RsxNode)) {
			if let RsxNode::Component(_) = node {
				return;
			}
			func(node);
			for child in node.children_mut() {
				inner(child, func);
			}
		}
		inner(self, &mut func);
	}
}

#[cfg(test)]
mod test {
	use super::RsxNodeVisitor;
	use crate::prelude::*;
	use sweet::prelude::*;



	struct Child;
	impl Component for Child {
		fn render(self) -> RsxRoot {
			rsx! {
				<div><slot /></div>
			}
		}
	}


	struct Counter {
		walk_element_children: bool,
		walk_component_node: bool,
		walk_component_slot_children: bool,
		//
		doctype: usize,
		comment: usize,
		text: usize,
		block: usize,
		component: usize,
		element: usize,
		attribute: usize,
	}
	impl Default for Counter {
		fn default() -> Self {
			Self {
				walk_element_children: true,
				walk_component_node: true,
				walk_component_slot_children: true,
				doctype: 0,
				comment: 0,
				text: 0,
				block: 0,
				component: 0,
				element: 0,
				attribute: 0,
			}
		}
	}

	#[allow(unused_variables)]
	impl RsxNodeVisitor for Counter {
		fn walk_element_children(&self) -> bool { self.walk_element_children }
		fn walk_component_node(&self) -> bool { self.walk_component_node }
		fn walk_component_slot_children(&self) -> bool {
			self.walk_component_slot_children
		}
		fn visit_doctype(&mut self, cx: &RsxContext) { self.doctype += 1; }
		fn visit_comment(&mut self, cx: &RsxContext, comment: &str) {
			self.comment += 1;
		}
		fn visit_text(&mut self, cx: &RsxContext, text: &str) {
			self.text += 1;
		}
		fn visit_block(&mut self, cx: &RsxContext, block: &RsxBlock) {
			self.block += 1;
		}
		fn visit_component(
			&mut self,
			cx: &RsxContext,
			component: &RsxComponent,
		) {
			self.component += 1;
		}
		fn visit_element(&mut self, cx: &RsxContext, element: &RsxElement) {
			// println!("visit element: {}", element.tag);
			self.element += 1;
		}
		fn visit_attribute(
			&mut self,
			cx: &RsxContext,
			attribute: &RsxAttribute,
		) {
			self.attribute += 1;
		}
	}



	#[test]
	fn test_visitor_counter() {
		let mut counter = Counter::default();
		rsx! {
			<!DOCTYPE html>
			<!-- "comment" -->
			<div class="test">
				"text"
				{7}
				<Child>
					<span />
				</Child>
			</div>
		}
		.walk(&mut counter);

		expect(counter.doctype).to_be(1);
		expect(counter.comment).to_be(1);
		expect(counter.text).to_be(1);
		expect(counter.block).to_be(1);
		expect(counter.element).to_be(4); // div + span + child div + child slot
		expect(counter.attribute).to_be(1); // class
		expect(counter.component).to_be(1); // Child
	}

	#[test]
	fn test_visitor_no_element_children() {
		let mut counter = Counter {
			walk_element_children: false,
			..Default::default()
		};

		rsx! {
			<div>
				<span />
			</div>
		}
		.walk(&mut counter);

		expect(counter.element).to_be(1); // just div, span not visited
	}

	#[test]
	fn test_visitor_no_component_node() {
		let mut counter = Counter {
			walk_component_node: false,
			..Default::default()
		};

		rsx! {
			<div>
				<Child>
					<span />
				</Child>
			</div>
		}
		.walk(&mut counter);

		expect(counter.component).to_be(1); // Child component
		expect(counter.element).to_be(1); // just div, Child's span not visited
	}

	#[test]
	fn visit_ignore_components() {
		let mut count = 0;
		rsx! {
			<div>
				<Child>
					<br />
				</Child>
				<br />
			</div>
		}
		.visit_ignore_components(|_| {
			count += 1;
		});
		expect(count).to_be(2);
	}
	#[test]
	fn visit_ignore_components_mut() {
		let mut count = 0;
		rsx! {
			<div>
				<Child>
					<br />
				</Child>
				<br />
			</div>
		}
		.visit_ignore_components_mut(|_| {
			count += 1;
		});
		expect(count).to_be(2);
	}
}
