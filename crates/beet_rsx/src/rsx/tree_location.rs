use crate::prelude::*;

pub type RustyIdx = u32;

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeLocation {
	/// Incremented every time an rsx node is encountered,
	/// used for reconciliation with the [TreeLocationMap::rusty_locations].
	/// It is required because not all rsx nodes are html nodes.
	pub tree_idx: TreeIdx,
	/// the [`TreeIdx`] of this node's parent *element*. This is used by
	/// text nodes to determine their location in the dom.
	pub parent_idx: TreeIdx,
	/// The *uncollapsed* child index of this node, for
	/// example the following has two child nodes, indexed
	/// as 0 and 1. When it is rendered they will be collapsed
	/// into a single text node, but we will split them up before
	/// using this index via the [TextBlockEncoder].
	///
	/// `<div> hello {"world"}</div>`
	pub child_idx: u32,
	// _padding: u32,
}

#[cfg(feature = "parser")]
impl quote::ToTokens for TreeLocation {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let tree_idx = *self.tree_idx;
		let parent_idx = *self.parent_idx;
		let child_idx = self.child_idx;
		tokens.extend(quote::quote! {
			TreeLocation::new(#tree_idx, #parent_idx, #child_idx)
		});
	}
}

impl TreeLocation {
	pub fn new(
		tree_idx: impl Into<TreeIdx>,
		parent_idx: impl Into<TreeIdx>,
		child_idx: u32,
	) -> Self {
		Self {
			tree_idx: tree_idx.into(),
			parent_idx: parent_idx.into(),
			child_idx,
		}
	}
}



/// Visit each node in a tree, this struct deliberately does not
/// provide an [`VisitRsxOptions`] as its essential that the tree is
/// walked in exactly the same way in all parts of beet for the locations
/// to be consistent.
#[derive(Debug)]
pub struct TreeLocationVisitor<Func> {
	/// Used by [`TreeLocation::parent_idx`].
	/// we use a stack because [RsxVisitor] is depth-first.
	/// This stack is a breadcrumb trail of parents
	parent_idxs: Vec<TreeIdx>,
	/// Used by [`TreeLocation::child_idx`].
	/// pushed when visiting children, incremented after visiting dom node
	child_idxs: Vec<u32>,
	/// Used by [`TreeLocation::tree_idx`].
	/// Simple counter that increments on each node visited.
	tree_idx_incr: u32,
	func: Func,
}




impl<Func> TreeLocationVisitor<Func> {
	pub fn new(func: Func) -> Self {
		Self::new_with_location(func, Default::default())
	}
	pub fn new_with_location(func: Func, location: TreeLocation) -> Self {
		Self {
			parent_idxs: vec![location.parent_idx],
			child_idxs: vec![location.child_idx],
			tree_idx_incr: *location.tree_idx,
			func,
		}
	}



	/// Visit a node and return the total number of elements visited
	pub fn visit(node: &RsxNode, func: Func)
	where
		Func: FnMut(TreeLocation, &RsxNode),
	{
		Self::new(func).walk_node(node);
	}

	pub fn visit_with_location(
		node: &RsxNode,
		location: TreeLocation,
		func: Func,
	) where
		Func: FnMut(TreeLocation, &RsxNode),
	{
		Self::new_with_location(func, location).walk_node(node);
	}
	pub fn visit_mut(node: &mut RsxNode, func: Func)
	where
		Func: FnMut(TreeLocation, &mut RsxNode),
	{
		Self::new(func).walk_node(node);
	}
	pub fn visit_with_location_mut(
		node: &mut RsxNode,
		location: TreeLocation,
		func: Func,
	) where
		Func: FnMut(TreeLocation, &mut RsxNode),
	{
		Self::new_with_location(func, location).walk_node(node);
	}

	/// Get the current item in the stack, or default
	/// # Panics
	/// Panics if the stack is empty
	// pub fn parent(&mut self) -> &mut TreeLocation {
	// 	self.parents
	// 		.last_mut()
	// 		.expect("TreeLocationVisitor stack is empty")
	// }

	fn current_location(&self) -> TreeLocation {
		let parent_idx = self.parent_idxs.last().cloned().unwrap_or_default();
		let child_idx = self.child_idxs.last().cloned().unwrap_or_default();
		TreeLocation::new(self.tree_idx_incr, parent_idx, child_idx)
	}

	fn before_node(&mut self, node: &RsxNode) {
		self.tree_idx_incr += 1;

		// it is not allowed to perform tree location walk before
		// resolving slot children
		match node {
			RsxNode::Component(comp) => comp.slot_children.assert_empty(),
			_ => {}
		}
	}

	fn after_node(&mut self, node: &RsxNode) {
		if node.is_html_node() {
			if let Some(child_idx) = self.child_idxs.last_mut() {
				*child_idx += 1;
			}
		}
	}

	fn before_children(&mut self) {
		self.parent_idxs.push(TreeIdx::new(self.tree_idx_incr));
		self.child_idxs.push(0);
	}
	fn after_children(&mut self) {
		self.parent_idxs.pop();
		self.child_idxs.pop();
	}
}


impl<Func: FnMut(TreeLocation, &RsxNode)> RsxVisitor
	for TreeLocationVisitor<Func>
{
	fn visit_node(&mut self, node: &RsxNode) {
		self.before_node(node);
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
impl<Func: FnMut(TreeLocation, &mut RsxNode)> RsxVisitorMut
	for TreeLocationVisitor<Func>
{
	fn visit_node(&mut self, node: &mut RsxNode) {
		self.before_node(node);
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
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let bucket = mock_bucket();
		let bucket2 = bucket.clone();
		let rsx = rsx! {
			// 0 - root
			<div>
				// 1 - child
				<div>
					// 2 - nested child
					<div />
					// 3 - second child
					<div />
				</div>
				// 4 - child 1
				<div />
			</div>
		};
		TreeLocationVisitor::visit(&rsx, move |loc, node| {
			if let RsxNode::Element(_) = node {
				bucket2.call(loc);
			}
		});
		expect(&bucket).to_have_been_called_times(5);
		// keep in mind that fragments will also increment
		// the tree_idx..
		expect(&bucket)
			.to_have_returned_nth_with(0, &TreeLocation::new(1, 0, 0));
		expect(&bucket)
			.to_have_returned_nth_with(1, &TreeLocation::new(3, 1, 0));
		expect(&bucket)
			// 2 because fragment
			.to_have_returned_nth_with(2, &TreeLocation::new(5, 3, 0));
		expect(&bucket)
			// 2 because fragment
			.to_have_returned_nth_with(3, &TreeLocation::new(7, 3, 1));
		expect(&bucket)
			.to_have_returned_nth_with(4, &TreeLocation::new(9, 1, 1));
	}


	#[cfg(feature = "parser")]
	#[test]
	fn to_tokens() {
		use quote::ToTokens;

		expect(TreeLocation::new(4, 2, 3).to_token_stream().to_string()).to_be(
			quote::quote! { TreeLocation::new(4u32, 2u32, 3u32) }.to_string(),
		);
	}


	#[test]
	#[should_panic]
	fn has_slot_children() {
		#[derive(Node)]
		struct Comp;

		fn comp(_: Comp) -> RsxNode {
			rsx! { <slot /> }
		}

		TreeLocationVisitor::visit(
			&rsx! {
				<Comp>
					<div />
				</Comp>
			},
			|_, _| {},
		);
	}
}
