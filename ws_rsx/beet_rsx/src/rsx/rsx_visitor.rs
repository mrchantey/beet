use crate::prelude::*;



#[derive(Default, Clone)]
pub struct VisitRsxOptions {
	/// do not visit [RsxBlock::initial]
	pub ignore_block_node_initial: bool,
	/// do not visit [RsxElement::children]
	pub ignore_element_children: bool,
	/// do not visit [RsxComponent::node]
	pub ignore_component_node: bool,
	/// do not visit [RsxComponent::slot_children]
	pub ignore_component_slot_children: bool,
	/// Visit all children before parents, this includes
	/// - fragment children
	/// - element children
	/// - component slot children
	/// - component node
	/// - block node initial
	pub bottom_up: bool,
	/// A predicate function to visit nodes
	pub should_visit_node: Option<fn(&RsxNode) -> bool>,
	/// A predicate function to visit the node of a component.
	/// This does not effect the slot children.
	pub should_visit_component_node: Option<fn(&RsxComponent) -> bool>,
}

pub const DEFAULT_VISIT_RSX_OPTIONS: VisitRsxOptions = VisitRsxOptions {
	ignore_block_node_initial: false,
	ignore_element_children: false,
	ignore_component_node: false,
	ignore_component_slot_children: false,
	bottom_up: false,
	should_visit_node: None,
	should_visit_component_node: None,
};

impl VisitRsxOptions {
	/// visit all children before parents, see `[VisitRsxOptions::bottom_up]`
	pub fn bottom_up() -> Self {
		Self {
			bottom_up: true,
			..Default::default()
		}
	}

	/// do not visit any nodes aside from direct child and fragments
	pub fn ignore_all() -> Self {
		Self {
			ignore_block_node_initial: true,
			ignore_element_children: true,
			ignore_component_node: true,
			ignore_component_slot_children: true,
			..Default::default()
		}
	}

	/// do not visit [RsxBlock::initial]
	pub fn ignore_block_node_initial() -> Self {
		Self {
			ignore_block_node_initial: true,
			..Default::default()
		}
	}
	/// do not visit [RsxElement::children]
	pub fn ignore_element_children() -> Self {
		Self {
			ignore_element_children: true,
			..Default::default()
		}
	}
	/// - do not visit [RsxComponent::root]
	/// - do not visit [RsxComponent::slot_children]
	pub fn ignore_component() -> Self {
		Self {
			ignore_component_node: true,
			ignore_component_slot_children: true,
			..Default::default()
		}
	}
	/// do not visit [RsxComponent::root]
	pub fn ignore_component_node() -> Self {
		Self {
			ignore_component_node: true,
			..Default::default()
		}
	}
	/// do not visit [RsxComponent::slot_children]
	pub fn ignore_component_slot_children() -> Self {
		Self {
			ignore_component_slot_children: true,
			..Default::default()
		}
	}
	pub fn should_visit_node(predicate: fn(&RsxNode) -> bool) -> Self {
		Self {
			should_visit_node: Some(predicate),
			..Default::default()
		}
	}
	pub fn should_visit_component_node(
		predicate: fn(&RsxComponent) -> bool,
	) -> Self {
		Self {
			should_visit_component_node: Some(predicate),
			..Default::default()
		}
	}
	/// Returns true if there is no predicate,
	/// otherwise returns the result of the predicate function
	pub fn passes_visit_node(&self, node: &RsxNode) -> bool {
		self.should_visit_node
			.map(|predicate| predicate(node))
			.unwrap_or(true)
	}
	/// Checks both `ignore_component_node` and `should_visit_component_node`
	pub fn passes_visit_component_node(&self, node: &RsxComponent) -> bool {
		!self.ignore_component_node
			&& self
				.should_visit_component_node
				.map(|predicate| predicate(node))
				.unwrap_or(true)
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
	fn visit_node(&mut self, node: &RsxNode) {}
	fn visit_doctype(&mut self, doctype: &RsxDoctype) {}
	fn visit_comment(&mut self, comment: &RsxComment) {}
	fn visit_text(&mut self, text: &RsxText) {}
	fn visit_block(&mut self, block: &RsxBlock) {}
	fn visit_component(&mut self, component: &RsxComponent) {}
	fn visit_fragment(&mut self, fragment: &RsxFragment) {}
	fn visit_element(&mut self, element: &RsxElement) {}
	fn visit_attribute(&mut self, attribute: &RsxAttribute) {}
	fn before_element_children(&mut self, element: &RsxElement) {}
	fn after_element_children(&mut self, element: &RsxElement) {}
	/// call the visit methods without walking any inners
	fn call_visit(&mut self, node: &RsxNode) {
		self.visit_node(node);
		match node {
			RsxNode::Doctype(doctype) => {
				self.visit_doctype(doctype);
			}
			RsxNode::Comment(comment) => self.visit_comment(comment),
			RsxNode::Text(text) => self.visit_text(text),
			RsxNode::Block(block) => {
				self.visit_block(block);
			}
			RsxNode::Fragment(fragment) => {
				self.visit_fragment(fragment);
			}
			RsxNode::Element(element) => {
				self.visit_element(element);
				for attr in &element.attributes {
					self.visit_attribute(attr);
				}
			}
			RsxNode::Component(component) => {
				self.visit_component(component);
			}
		}
	}

	/// walk the node tree, visiting each node
	/// either top down or bottom up
	fn walk_node(&mut self, node: &RsxNode) {
		if !self.options().passes_visit_node(node) {
			return;
		}

		// if top down, visit before walk
		if self.options().bottom_up == false {
			self.call_visit(node);
		}
		match node {
			RsxNode::Fragment(fragment) => {
				for child in &fragment.nodes {
					self.walk_node(child);
				}
			}
			RsxNode::Block(b) => {
				if !self.options().ignore_block_node_initial {
					self.walk_node(&b.initial);
				}
			}
			RsxNode::Element(element) => {
				if !self.options().ignore_element_children {
					self.before_element_children(element);
					self.walk_node(&element.children);
					self.after_element_children(element);
				}
			}
			RsxNode::Component(component) => {
				if self.options().passes_visit_component_node(component) {
					self.walk_node(&component.node);
				}
				if !self.options().ignore_component_slot_children {
					self.walk_node(&component.slot_children);
				}
			}
			_ => {}
		}
		// if bottom up, visit after walk
		if self.options().bottom_up == true {
			self.call_visit(node);
		}
	}
}

/// See [`RsxNodeVisitor`]
#[allow(unused_variables)]
pub trait RsxVisitorMut {
	fn options(&self) -> &VisitRsxOptions { &DEFAULT_VISIT_RSX_OPTIONS }
	fn visit_node(&mut self, node: &mut RsxNode) {}
	fn visit_doctype(&mut self, doctype: &mut RsxDoctype) {}
	fn visit_comment(&mut self, comment: &mut RsxComment) {}
	fn visit_text(&mut self, text: &mut RsxText) {}
	fn visit_block(&mut self, block: &mut RsxBlock) {}
	fn visit_component(&mut self, component: &mut RsxComponent) {}
	fn visit_fragment(&mut self, fragment: &mut RsxFragment) {}
	fn visit_element(&mut self, element: &mut RsxElement) {}
	fn visit_attribute(&mut self, attribute: &mut RsxAttribute) {}
	fn before_element_children(&mut self, element: &mut RsxElement) {}
	fn after_element_children(&mut self, element: &mut RsxElement) {}
	/// call the visit methods without walking any inners
	fn call_visit(&mut self, node: &mut RsxNode) {
		self.visit_node(node);
		match node {
			RsxNode::Doctype(doctype) => {
				self.visit_doctype(doctype);
			}
			RsxNode::Comment(comment) => self.visit_comment(comment),
			RsxNode::Text(text) => self.visit_text(text),
			RsxNode::Block(block) => {
				self.visit_block(block);
			}
			RsxNode::Fragment(fragment) => {
				self.visit_fragment(fragment);
			}
			RsxNode::Element(element) => {
				self.visit_element(element);
				for attr in &mut element.attributes {
					self.visit_attribute(attr);
				}
			}
			RsxNode::Component(component) => {
				self.visit_component(component);
			}
		}
	}
	/// walk the node tree, visiting each node
	/// either top down or bottom up
	fn walk_node(&mut self, node: &mut RsxNode) {
		if !self.options().passes_visit_node(node) {
			return;
		}

		// if top down, visit before walk
		if self.options().bottom_up == false {
			self.call_visit(node);
		}
		match node {
			RsxNode::Fragment(fragment) => {
				for child in &mut fragment.nodes {
					self.walk_node(child);
				}
			}
			RsxNode::Block(b) => {
				if !self.options().ignore_block_node_initial {
					self.walk_node(&mut b.initial);
				}
			}
			RsxNode::Element(element) => {
				if !self.options().ignore_element_children {
					self.before_element_children(element);
					self.walk_node(&mut element.children);
					self.after_element_children(element);
				}
			}
			RsxNode::Component(component) => {
				if self.options().passes_visit_component_node(component) {
					self.walk_node(&mut component.node);
				}
				if !self.options().ignore_component_slot_children {
					self.walk_node(&mut component.slot_children);
				}
			}
			_ => {}
		}
		// if bottom up, visit after walk
		if self.options().bottom_up == true {
			self.call_visit(node);
		}
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[derive(Node)]
	struct Child;

	fn child(_: Child) -> RsxNode {
		rsx! {
			<div>
				<slot />
			</div>
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
		fn visit_doctype(&mut self, doctype: &RsxDoctype) { self.doctype += 1; }
		fn visit_comment(&mut self, comment: &RsxComment) { self.comment += 1; }
		fn visit_text(&mut self, text: &RsxText) { self.text += 1; }
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
			// doctype
			<!DOCTYPE html>
			// comment
			<!-- "comment" -->
			// attribute
			<div class="test">
				// text node
				// block node
				// {child_block}					// block child
				// component
				"text" {7} <Child>
					// component child
					<span />
					// nested component
					<Child>
						// nested child
						<span />
					</Child>
				</Child>
			</div>
		}
		.xpipe(ApplySlots::default())
		.unwrap()
		.walk(&mut counter);
		expect(counter.node).to_be(22);
		expect(counter.doctype).to_be(1);
		expect(counter.comment).to_be(1);
		expect(counter.text).to_be(2);
		expect(counter.block).to_be(1);
		expect(counter.element).to_be(5); // div + span + child div
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
		expect(counter.element).to_be(2); // just div and span, Child's div not visited
	}

	#[test]
	fn top_down() {
		let node = rsx! {
			<div val="1">
				<div val="2" />
			</div>
		};

		let mut counts = Vec::new();
		VisitRsxAttribute::walk_with_opts(
			&node,
			VisitRsxOptions {
				..Default::default()
			},
			|attr| {
				let RsxAttribute::KeyValue { value, .. } = attr else {
					return;
				};
				counts.push(value.parse::<u32>().unwrap());
			},
		);
		expect(counts).to_be(vec![1, 2]);
	}
	#[test]
	fn bottom_up() {
		let node = rsx! {
			<div val="1">
				<div val="2" />
			</div>
		};

		let mut counts = Vec::new();
		VisitRsxAttribute::walk_with_opts(
			&node,
			VisitRsxOptions {
				bottom_up: true,
				..Default::default()
			},
			|attr| {
				let RsxAttribute::KeyValue { value, .. } = attr else {
					return;
				};
				counts.push(value.parse::<u32>().unwrap());
			},
		);
		expect(counts).to_be(vec![2, 1]);
	}
}
