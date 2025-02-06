use crate::prelude::*;


#[derive(Debug, Default, Clone)]
pub struct VisitRsxOptions {
	/// do not visit the initial block node
	pub ignore_block_node_initial: bool,
	/// do not visit the children of an element
	pub ignore_element_children: bool,
	/// do not visit the component *node*
	pub ignore_component_node: bool,
	/// do not visit the component *slot children*
	pub ignore_component_slot_children: bool,
}

pub const DEFAULT_VISIT_RSX_OPTIONS: VisitRsxOptions = VisitRsxOptions {
	ignore_block_node_initial: false,
	ignore_element_children: false,
	ignore_component_node: false,
	ignore_component_slot_children: false,
};

impl VisitRsxOptions {
	pub fn ignore_block_node_initial() -> Self {
		Self {
			ignore_block_node_initial: true,
			..Default::default()
		}
	}
	pub fn ignore_element_children() -> Self {
		Self {
			ignore_element_children: true,
			..Default::default()
		}
	}
	pub fn ignore_component_node() -> Self {
		Self {
			ignore_component_node: true,
			..Default::default()
		}
	}
	pub fn ignore_component_slot_children() -> Self {
		Self {
			ignore_component_slot_children: true,
			..Default::default()
		}
	}
}

///
/// Walking trees like rsx is deceptively difficult.
/// The visitor pattern handles the 'walking' and allows implementers
/// to focus on the 'visiting'.
///
/// This implementation is depth-first call stack based,
/// visiting parent elements, components and blocks before walking children.
///
/// Visiting fragments is intentionally not supported,
/// they are by definition transparent so depending on them
/// is considered an antipattern.
#[allow(unused_variables)]
pub trait RsxVisitor {
	/// get the options
	fn options(&self) -> &VisitRsxOptions { &DEFAULT_VISIT_RSX_OPTIONS }
	/// override the options
	fn ignore_block_node_initial(&self) -> bool {
		self.options().ignore_block_node_initial
	}
	/// override the options
	fn ignore_element_children(&self) -> bool {
		self.options().ignore_element_children
	}
	/// override the options
	fn ignore_component_node(&self) -> bool {
		self.options().ignore_component_node
	}
	/// override the options
	fn ignore_component_slot_children(&self) -> bool {
		self.options().ignore_component_slot_children
	}
	fn visit_node(&mut self, node: &RsxNode) {}
	fn visit_doctype(&mut self) {}
	fn visit_comment(&mut self, comment: &str) {}
	fn visit_text(&mut self, text: &str) {}
	fn visit_block(&mut self, block: &RsxBlock) {}
	fn visit_component(&mut self, component: &RsxComponent) {}
	fn visit_element(&mut self, element: &RsxElement) {}
	fn visit_attribute(&mut self, attribute: &RsxAttribute) {}
	fn before_element_children(&mut self, element: &RsxElement) {}
	fn after_element_children(&mut self, element: &RsxElement) {}
	fn walk_node(&mut self, node: &RsxNode) {
		self.visit_node(node);
		match node {
			RsxNode::Doctype => {
				self.visit_doctype();
			}
			RsxNode::Comment(c) => self.visit_comment(c),
			RsxNode::Text(t) => self.visit_text(t),
			RsxNode::Block(b) => {
				self.visit_block(b);
				if !self.ignore_block_node_initial() {
					self.walk_node(&b.initial);
				}
			}
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
			self.before_element_children(element);
			for child in &element.children {
				self.walk_node(child);
			}
			self.after_element_children(element);
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
	fn options(&self) -> &VisitRsxOptions { &DEFAULT_VISIT_RSX_OPTIONS }
	fn ignore_block_node_initial(&self) -> bool {
		self.options().ignore_block_node_initial
	}
	fn ignore_element_children(&self) -> bool {
		self.options().ignore_element_children
	}
	fn ignore_component_node(&self) -> bool {
		self.options().ignore_component_node
	}
	fn ignore_component_slot_children(&self) -> bool {
		self.options().ignore_component_slot_children
	}
	fn visit_node(&mut self, node: &mut RsxNode) {}
	fn visit_doctype(&mut self) {}
	fn visit_comment(&mut self, comment: &mut str) {}
	fn visit_text(&mut self, text: &mut str) {}
	fn visit_block(&mut self, block: &mut RsxBlock) {}
	fn visit_component(&mut self, component: &mut RsxComponent) {}
	fn visit_element(&mut self, element: &mut RsxElement) {}
	fn visit_attribute(&mut self, attribute: &mut RsxAttribute) {}
	fn before_element_children(&mut self, element: &mut RsxElement) {}
	fn after_element_children(&mut self, element: &mut RsxElement) {}
	fn walk_node(&mut self, node: &mut RsxNode) {
		self.visit_node(node);
		match node {
			RsxNode::Doctype => {
				self.visit_doctype();
			}
			RsxNode::Comment(c) => self.visit_comment(c),
			RsxNode::Text(t) => self.visit_text(t),
			RsxNode::Block(b) => {
				self.visit_block(b);
				if !self.ignore_block_node_initial() {
					self.walk_node(&mut b.initial);
				}
			}
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
			self.before_element_children(element);
			for child in &mut element.children {
				self.walk_node(child);
			}
			self.after_element_children(element);
		}
	}
	fn walk_component(&mut self, component: &mut RsxComponent) {
		self.visit_component(component);
		if !self.ignore_component_node() {
			self.walk_node(&mut component.node);
		}
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
		options: VisitRsxOptions,
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
		fn options(&self) -> &VisitRsxOptions { &self.options }
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
		// let child_block = rsx! { <div> {"text"} </div> };

		let mut counter = Counter::default();
		rsx! {
			<!DOCTYPE html>					// doctype
			<!-- "comment" -->			// comment
			<div class="test">			// attribute
				"text"								// text node
				{7}										// block node
				// {child_block}					// block child
				<Child>								// component
					<span />						// component child
					<Child> 						// nested component
						<span />					// nested child
					</Child>
				</Child>
			</div>
		}
		.walk(&mut counter);

		expect(counter.node).to_be(15);
		expect(counter.doctype).to_be(1);
		expect(counter.comment).to_be(1);
		expect(counter.text).to_be(2);
		expect(counter.block).to_be(1);
		expect(counter.element).to_be(7); // div + span + child div + child slot
		expect(counter.attribute).to_be(1); // class
		expect(counter.component).to_be(2); // Child
	}

	#[test]
	fn test_visitor_no_element_children() {
		let mut counter = Counter {
			options: VisitRsxOptions::ignore_element_children(),
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
			options: VisitRsxOptions::ignore_component_node(),
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
}
