use crate::prelude::*;

pub type RustyIdx = u32;

/// Used to identify a html node.
/// Incremented via dfs
pub type DomIdx = u32;
/// Used to identify a rsx node.
/// Incremented via dfs
pub type RsxIdx = u32;


///	This struct is the binding between a [RsxNode] and an [HtmlNode].
///
/// Hydrating elements is relatively simple, we can just slap an id on them,
/// but text nodes don't have ids, and to make things even more exciting adjacent
/// nodes are collapsed when rendered.
///
/// ## Footgun
/// These indices are *uncollapsed* indices.
/// When we render html adjacent text nodes are collapsed into a single text node.
/// We use [TextBlockEncoder] to track this behavior, and re-expand the text nodes
/// before using this location.
///
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomLocation {
	/// the *uncollapsed* index of the current node.
	/// This is unique for every html node in a tree.
	pub dom_idx: DomIdx,
	/// the index of this node's parent *element*. This is used by
	/// text nodes to determine their location in the dom.
	pub parent_idx: DomIdx,
	/// The *uncollapsed* child index of this node, for
	/// example the following has two child nodes, indexed
	/// as 0 and 1. When it is rendered they will be collapsed
	/// into a single text node, but we will split them up before
	/// using this index via the [TextBlockEncoder].
	///
	/// `<div> hello {"world"}</div>`
	pub child_idx: u32,
	/// Incremented every time an rsx node is encountered,
	/// used for reconciliation with the [DomLocationMap::rusty_locations].
	/// It is required because not all rsx nodes are html nodes.
	pub rsx_idx: RsxIdx,
}

impl DomLocation {
	pub fn to_csv(&self) -> String {
		// must keep in sync with from_csv
		vec![
			self.dom_idx.to_string(),
			self.parent_idx.to_string(),
			self.child_idx.to_string(),
			self.rsx_idx.to_string(),
		]
		.join(",")
	}
	pub fn from_csv(csv: &str) -> ParseResult<Self> {
		let mut parts = csv.split(',');
		let mut next = || -> Result<u32, ParseError> {
			let next = parts
				.next()
				.ok_or_else(|| ParseError::serde("invalid rsx context csv"))?
				.parse()?;
			Ok(next)
		};

		// must keep in sync with to_csv
		let dom_idx = next()?;
		let parent_idx = next()?;
		let child_idx = next()?;
		let rsx_idx = next()?;

		Ok(Self {
			dom_idx,
			parent_idx,
			child_idx,
			rsx_idx,
		})
	}
}



/// Wrapper of a visitor but
#[derive(Debug)]
pub struct DomLocationVisitor<Func> {
	/// we use a stack because [RsxVisitor] is depth-first.
	/// This stack is an immutable breadcrumb trail of parents
	parent_idxs: Vec<u32>,
	/// pushed when visiting children, incremented after visiting dom node
	child_idxs: Vec<u32>,
	dom_idx_incr: u32,
	rsx_idx_incr: u32,
	options: VisitRsxOptions,
	func: Func,
}
impl<Func> DomLocationVisitor<Func> {
	/// Visit a node and return the total number of elements visited
	pub fn visit(node: &RsxNode, func: Func)
	where
		Func: FnMut(DomLocation, &RsxNode),
	{
		Self {
			parent_idxs: vec![Default::default()],
			child_idxs: vec![Default::default()],
			dom_idx_incr: 0,
			rsx_idx_incr: 0,
			options: Default::default(),
			func,
		}
		.walk_node(node);
	}

	pub fn visit_with_options(
		node: &RsxNode,
		options: VisitRsxOptions,
		func: Func,
	) where
		Func: FnMut(DomLocation, &RsxNode),
	{
		Self {
			parent_idxs: vec![Default::default()],
			child_idxs: vec![Default::default()],
			dom_idx_incr: 0,
			rsx_idx_incr: 0,
			options,
			func,
		}
		.walk_node(node);
	}
	pub fn visit_mut(node: &mut RsxNode, func: Func)
	where
		Func: FnMut(DomLocation, &mut RsxNode),
	{
		Self {
			parent_idxs: vec![Default::default()],
			child_idxs: vec![Default::default()],
			dom_idx_incr: 0,
			rsx_idx_incr: 0,
			options: Default::default(),
			func,
		}
		.walk_node(node);
	}
	pub fn visit_with_options_mut(
		node: &mut RsxNode,
		options: VisitRsxOptions,
		func: Func,
	) where
		Func: FnMut(DomLocation, &mut RsxNode),
	{
		Self {
			parent_idxs: Default::default(),
			child_idxs: Default::default(),
			dom_idx_incr: 0,
			rsx_idx_incr: 0,
			options,
			func,
		}
		.walk_node(node);
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
			dom_idx: self.dom_idx_incr,
			rsx_idx: self.rsx_idx_incr,
			parent_idx,
			child_idx,
		}
	}
	pub fn after_node(&mut self, node: &RsxNode) {
		self.rsx_idx_incr += 1;
		if node.is_html_node() {
			if let Some(child_idx) = self.child_idxs.last_mut() {
				*child_idx += 1;
			}
			self.dom_idx_incr += 1;
		}
	}

	pub fn before_children(&mut self) {
		// idx was incremented after visit node so subtract one 
		// to get parent idx
		// no need to saturating sub because we must've already visited node
		self.parent_idxs.push(self.dom_idx_incr - 1);
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
	fn csv() {
		let a = DomLocation {
			dom_idx: 1,
			parent_idx: 2,
			child_idx: 3,
			rsx_idx: 4,
		};
		let csv = a.to_csv();
		let b = DomLocation::from_csv(&csv).unwrap();
		expect(a).to_be(b);
	}
	#[test]
	fn works() {
		let bucket = mock_bucket();
		let bucket2 = bucket.clone();
		let rsx = rsx! {
			   <div>				// 0 - root
				   <div>			// 1 - child
					   <div/>		// 2 - nested child
					   <div/>		// 3 - second child
				   </div>
				   <div/>			// 4 - child 1
			   </div>
		};
		DomLocationVisitor::visit(&rsx, move |loc, _| {
			bucket2.call(loc);
		});
		expect(&bucket).to_have_returned_nth_with(0, &DomLocation {
			dom_idx: 0,
			parent_idx: 0,
			child_idx: 0,
			rsx_idx: 0,
		});
		expect(&bucket).to_have_returned_nth_with(1, &DomLocation {
			dom_idx: 1,
			parent_idx: 0,
			child_idx: 0,
			rsx_idx: 1,
		});
		expect(&bucket).to_have_returned_nth_with(2, &DomLocation {
			dom_idx: 2,
			parent_idx: 1,
			child_idx: 0,
			rsx_idx: 2,
		});
		expect(&bucket).to_have_returned_nth_with(3, &DomLocation {
			dom_idx: 3,
			parent_idx: 1,
			child_idx: 1,
			rsx_idx: 3,
		});
		expect(&bucket).to_have_returned_nth_with(4, &DomLocation {
			dom_idx: 4,
			parent_idx: 0,
			child_idx: 1,
			rsx_idx: 4,
		});
	}
}
