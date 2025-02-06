use crate::prelude::*;

/// For hydration we need to know the location of a node in the dom.
/// Hydrating elements is relatively simple, we can just slap an id on them,
/// but text nodes don't have ids, and to make things even more exciting adjacent
/// nodes are collapsed when rendered.
///
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomLocation {
	/// the index of the current element, for text nodes
	/// this is the same as `parent_idx`
	pub element_idx: u32,
	/// the index of this node's parent. This is used by
	/// text nodes to determine their location in the dom.
	pub parent_idx: u32,
	/// The *pre-collapsed* child index of this node, for
	/// example the following has two child nodes, indexed
	/// as 0 and 1. When it is rendered they will be collapsed
	/// into a single text node, but we will split them up before
	/// using this index via the [TextBlockEncoder].
	///
	/// `<div> hello {"world"}</div>`
	pub child_idx: u32,
}

impl DomLocation {}



/// Wrapper of a visitor but
#[derive(Debug)]
pub struct DomLocationVisitor<Func> {
	/// we use a stack because [RsxVisitor] is depth-first.
	/// This stack is an immutable breadcrumb trail of parents
	parent_idxs: Vec<u32>,
	child_idxs: Vec<u32>,
	curr_element_idx: u32,
	options: VisitRsxOptions,
	func: Func,
}
impl<Func> DomLocationVisitor<Func> {
	pub fn new(func: Func) -> Self
	where
		Func: FnMut(DomLocation, &RsxNode),
	{
		Self {
			parent_idxs: vec![Default::default()],
			child_idxs: vec![Default::default()],
			curr_element_idx: 0,
			options: Default::default(),
			func,
		}
	}
	pub fn new_with_options(options: VisitRsxOptions, func: Func) -> Self
	where
		Func: FnMut(DomLocation, &RsxNode),
	{
		Self {
			parent_idxs: vec![Default::default()],
			child_idxs: vec![Default::default()],
			curr_element_idx: 0,
			options,
			func,
		}
	}
	pub fn new_mut(func: Func) -> Self
	where
		Func: FnMut(DomLocation, &mut RsxNode),
	{
		Self {
			parent_idxs: vec![Default::default()],
			child_idxs: vec![Default::default()],
			curr_element_idx: 0,
			options: Default::default(),
			func,
		}
	}
	pub fn new_with_options_mut(options: VisitRsxOptions, func: Func) -> Self
	where
		Func: FnMut(DomLocation, &mut RsxNode),
	{
		Self {
			parent_idxs: Default::default(),
			child_idxs: Default::default(),
			curr_element_idx: 0,
			options,
			func,
		}
	}

	/// Get the current item in the stack, or default
	/// # Panics
	/// Panics if the stack is empty
	// pub fn parent(&mut self) -> &mut DomLocation {
	// 	self.parents
	// 		.last_mut()
	// 		.expect("DomLocationVisitor stack is empty")
	// }

	pub fn current_location(&self) -> DomLocation {
		let parent_idx = self.parent_idxs.last().cloned().unwrap_or_default();
		let child_idx = self.child_idxs.last().cloned().unwrap_or_default();
		DomLocation {
			element_idx: self.curr_element_idx,
			parent_idx,
			child_idx,
		}
	}
	pub fn after_node(&mut self, node: &RsxNode) {
		if node.is_html_node() {
			if let Some(child_idx) = self.child_idxs.last_mut() {
				*child_idx += 1;
			}
			self.curr_element_idx += 1;
		}
	}

	pub fn before_children(&mut self) {
		// idx was incremented after visit so subtract one
		// no need to saturating sub because we must've already visited node
		self.parent_idxs.push(self.curr_element_idx - 1);
		self.child_idxs.push(0);
	}
	pub fn after_children(&mut self) {
		self.parent_idxs.pop();
		self.child_idxs.pop();
	}
}


impl<Func: FnMut(DomLocation, &RsxNode)> RsxVisitor
	for DomLocationVisitor<Func>
{
	fn options(&self) -> &VisitRsxOptions { &self.options }
	fn visit_node(&mut self, node: &RsxNode) {
		let loc = self.current_location();
		(self.func)(loc, node);
		self.after_node(node);
	}
	fn before_element_children(&mut self, _: &RsxElement) {
		self.before_children();
	}
	fn after_element_children(&mut self, _: &RsxElement) {
		self.after_children();
	}
}
impl<Func: FnMut(DomLocation, &mut RsxNode)> RsxVisitorMut
	for DomLocationVisitor<Func>
{
	fn options(&self) -> &VisitRsxOptions { &self.options }
	fn visit_node(&mut self, node: &mut RsxNode) {
		let loc = self.current_location();
		(self.func)(loc, node);
		self.after_node(node);
	}
	fn before_element_children(&mut self, _: &mut RsxElement) {
		self.before_children();
	}
	fn after_element_children(&mut self, _: &mut RsxElement) {
		self.after_children();
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let bucket = mock_bucket();
		let bucket2 = bucket.clone();
		DomLocationVisitor::new(move |loc, _| {
			bucket2.call(loc);
		})
		.walk_node(
			&rsx! {
			   <div>				// 0 - root
				   <div>			// 1 - child
					   <div/>		// 2 - nested child
					   <div/>		// 3 - second child
				   </div>
				   <div/>			// 4 - child 1
			   </div>
			}
			.node,
		);
		expect(&bucket).to_have_returned_nth_with(0, &DomLocation {
			element_idx: 0,
			parent_idx: 0,
			child_idx: 0,
		});
		expect(&bucket).to_have_returned_nth_with(1, &DomLocation {
			element_idx: 1,
			parent_idx: 0,
			child_idx: 0,
		});
		expect(&bucket).to_have_returned_nth_with(2, &DomLocation {
			element_idx: 2,
			parent_idx: 1,
			child_idx: 0,
		});
		expect(&bucket).to_have_returned_nth_with(3, &DomLocation {
			element_idx: 3,
			parent_idx: 1,
			child_idx: 1,
		});
		expect(&bucket).to_have_returned_nth_with(4, &DomLocation {
			element_idx: 4,
			parent_idx: 0,
			child_idx: 1,
		});
	}
}
