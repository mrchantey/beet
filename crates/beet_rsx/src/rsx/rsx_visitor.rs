use crate::prelude::*;


/// Walking trees like rsx is deceptively difficult.
/// The visitor pattern handles the 'walking' and allows implementers
/// to focus on the 'visiting'.
///
/// Visiting fragments is intentionally not supported,
/// they are by definition transparent so depending on them
/// is considered an antipattern.
#[allow(unused_variables)]
pub trait RsxVisitor {
	fn ignore_element_children(&self) -> bool { false }
	fn ignore_component_node(&self) -> bool { false }
	fn ignore_component_slot_children(&self) -> bool { false }
	fn visit_node(&mut self, node: &RsxNode) {}
	fn visit_doctype(&mut self) {}
	fn visit_comment(&mut self, comment: &str) {}
	fn visit_text(&mut self, text: &str) {}
	fn visit_block(&mut self, block: &RsxBlock) {}
	fn visit_component(&mut self, component: &RsxComponent) {}
	fn visit_element(&mut self, element: &RsxElement) {}
	fn visit_attribute(&mut self, attribute: &RsxAttribute) {}
	fn walk_node(&mut self, node: &RsxNode) {
		self.visit_node(node);
		match node {
			RsxNode::Doctype => {
				self.visit_doctype();
			}
			RsxNode::Comment(c) => self.visit_comment(c),
			RsxNode::Text(t) => self.visit_text(t),
			RsxNode::Block(b) => self.visit_block(b),
			RsxNode::Fragment(f) => {
				for child in f {
					self.walk_node(child);
				}
			}
			RsxNode::Element(e) => {
				self.walk_element(e);
			}
			RsxNode::Component(c) => {
				self.walk_component(c);
			}
		}
	}
	fn walk_element(&mut self, element: &RsxElement) {
		self.visit_element(element);
		for attr in &element.attributes {
			self.visit_attribute(attr);
		}
		if !self.ignore_element_children() {
			for child in &element.children {
				self.walk_node(child);
			}
		}
	}
	fn walk_component(&mut self, component: &RsxComponent) {
		self.visit_component(component);
		if !self.ignore_component_node() {
			self.walk_node(&component.node);
		}
	}
}

/// See [`RsxNodeVisitor`]
#[allow(unused_variables)]
pub trait RsxVisitorMut {
	fn ignore_element_children(&self) -> bool { false }
	fn ignore_component_node(&self) -> bool { false }
	fn ignore_component_slot_children(&self) -> bool { false }
	fn visit_node(&mut self, node: &mut RsxNode) {}
	fn visit_doctype(&mut self) {}
	fn visit_comment(&mut self, comment: &mut str) {}
	fn visit_text(&mut self, text: &mut str) {}
	fn visit_block(&mut self, block: &mut RsxBlock) {}
	fn visit_component(&mut self, component: &mut RsxComponent) {}
	fn visit_element(&mut self, element: &mut RsxElement) {}
	fn visit_attribute(&mut self, attribute: &mut RsxAttribute) {}
	fn walk_node(&mut self, node: &mut RsxNode) {
		self.visit_node(node);
		match node {
			RsxNode::Doctype => {
				self.visit_doctype();
			}
			RsxNode::Comment(c) => self.visit_comment(c),
			RsxNode::Text(t) => self.visit_text(t),
			RsxNode::Block(b) => self.visit_block(b),
			RsxNode::Fragment(f) => {
				for child in f {
					self.walk_node(child);
				}
			}
			RsxNode::Element(e) => {
				self.walk_element(e);
			}
			RsxNode::Component(c) => {
				self.walk_component(c);
			}
		}
	}
	fn walk_element(&mut self, element: &mut RsxElement) {
		self.visit_element(element);
		for attr in &mut element.attributes {
			self.visit_attribute(attr);
		}
		if !self.ignore_element_children() {
			for child in &mut element.children {
				self.walk_node(child);
			}
		}
	}
	fn walk_component(&mut self, component: &mut RsxComponent) {
		self.visit_component(component);
		if !self.ignore_component_node() {
			self.walk_node(&mut component.node);
		}
	}
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
	use super::RsxVisitor;
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


	#[derive(Default)]
	struct Counter {
		ignore_element_children: bool,
		ignore_component_node: bool,
		ignore_component_slot_children: bool,
		//
		node: usize,
		doctype: usize,
		comment: usize,
		text: usize,
		block: usize,
		component: usize,
		element: usize,
		attribute: usize,
	}

	#[allow(unused_variables)]
	impl RsxVisitor for Counter {
		fn ignore_element_children(&self) -> bool {
			self.ignore_element_children
		}
		fn ignore_component_node(&self) -> bool { self.ignore_component_node }
		fn ignore_component_slot_children(&self) -> bool {
			self.ignore_component_slot_children
		}
		fn visit_node(&mut self, node: &RsxNode) { self.node += 1; }
		fn visit_doctype(&mut self) { self.doctype += 1; }
		fn visit_comment(&mut self, comment: &str) { self.comment += 1; }
		fn visit_text(&mut self, text: &str) { self.text += 1; }
		fn visit_block(&mut self, block: &RsxBlock) { self.block += 1; }
		fn visit_component(&mut self, component: &RsxComponent) {
			self.component += 1;
		}
		fn visit_element(&mut self, element: &RsxElement) {
			// println!("visit element: {}", element.tag);
			self.element += 1;
		}
		fn visit_attribute(&mut self, attribute: &RsxAttribute) {
			self.attribute += 1;
		}
	}



	#[test]
	fn test_visitor_counter() {
		let mut counter = Counter::default();
		rsx! {
			<!DOCTYPE html>					// doctype
			<!-- "comment" -->			// comment
			<div class="test">			// attribute
				"text"								// text node
				{7}										// block node
				<Child>								// component
					<span />						// component child
					<Child> 						// nested component
						<span />					// nested child
					</Child>
				</Child>
			</div>
		}
		.walk(&mut counter);

		expect(counter.node).to_be(14);
		expect(counter.doctype).to_be(1);
		expect(counter.comment).to_be(1);
		expect(counter.text).to_be(1);
		expect(counter.block).to_be(1);
		expect(counter.element).to_be(7); // div + span + child div + child slot
		expect(counter.attribute).to_be(1); // class
		expect(counter.component).to_be(2); // Child
	}

	#[test]
	fn test_visitor_no_element_children() {
		let mut counter = Counter {
			ignore_element_children: true,
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
			ignore_component_node: true,
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
