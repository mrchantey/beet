use crate::prelude::*;
use std::borrow::Borrow;
use std::collections::VecDeque;
use strum_macros::EnumDiscriminants;

/// Used to identify an element in a tree.
/// This is incremented in a breadth-first pattern
/// as we visit each element in the tree.
pub type ElementIdx = usize;
/// Used to identify a rust block in a tree.
/// This is key to being able to reconcile a changed html tree
/// with precompiled rust blocks.
/// This is incremented in a breadth-first pattern
/// as we visit each rust block in the tree.
pub type BlockIdx = usize;


/// A collection of indexes for working with rsx nodes,
/// this is particularly useful for resumability, hot reloading
/// etc where we need to reconcile a html page with rust blocks
/// for hydration.
/// # PartialEq
///
/// PartialEq may or may not behave as you expect,
/// for example visiting a hot-reloaded rsx struct and diffing it with
/// a precompiled rsx struct will not be equal.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RsxContext {
	/// Simple counter that increments after any [RsxNode] is visited.
	/// The nature of the algorithm means this will start with 1, so we
	/// saturating_sub for the getter
	pub(super) node_idx: usize,
	/// The number of components visited
	pub(super) component_idx: usize,
	/// the number of rsx rust blocks visited,
	/// this is useful for hot reloading because it will not change
	/// even if the html structure changes
	pub(super) block_idx: BlockIdx,
	/// Element count is a special case, it goes up and down,
	/// so should be zero by the time the tree is finished.
	/// In the case of a rust block this is the parent element.
	/// in the case of visiting an element this is the element itself.
	pub(super) element_count: ElementIdx,
	/// the *uncollapsed* index of this block relative to its parent element.
	/// That is the [RsxNode] child index, not the [HtmlNode] child index
	/// which merges rust text blocks with static text blocks
	pub(super) child_idx: usize,
}

impl RsxContext {
	pub fn node_idx(&self) -> usize { self.node_idx }
	pub fn component_idx(&self) -> usize { self.component_idx }
	pub fn block_idx(&self) -> usize { self.block_idx }
	pub fn element_idx(&self) -> usize { self.element_count.saturating_sub(1) }
	pub fn child_idx(&self) -> usize { self.child_idx }

	fn before_visit_node(
		&mut self,
		node_disc: &RsxNodeDiscriminants,
		pos_disc: &HtmlElementPositionDiscriminants,
	) {
		match node_disc {
			RsxNodeDiscriminants::Element => {
				self.element_count += 1;
			}
			_ => {}
		}
		match pos_disc {
			HtmlElementPositionDiscriminants::FirstChild
			| HtmlElementPositionDiscriminants::OnlyChild => {
				self.child_idx = 0;
			}
			_ => {}
		}
	}
	fn after_visit_node(
		&mut self,
		node_disc: &RsxNodeDiscriminants,
		pos_disc: &HtmlElementPositionDiscriminants,
	) {
		self.node_idx += 1;
		match node_disc {
			RsxNodeDiscriminants::Block => {
				self.block_idx += 1;
			}
			RsxNodeDiscriminants::Component => {
				self.component_idx += 1;
			}
			RsxNodeDiscriminants::Fragment => {}
			RsxNodeDiscriminants::Element
			| RsxNodeDiscriminants::Text
			| RsxNodeDiscriminants::Doctype
			| RsxNodeDiscriminants::Comment => {
				self.child_idx += 1;
			}
		}
		match pos_disc {
			HtmlElementPositionDiscriminants::LastChild
			| HtmlElementPositionDiscriminants::OnlyChild => {
				// root parent may be a fragment so saturate
				self.element_count = self.element_count.saturating_sub(1);
			}
			_ => {}
		}
	}


	pub fn to_csv(&self) -> String {
		// must keep in sync with from_csv
		vec![
			self.node_idx.to_string(),
			self.component_idx.to_string(),
			self.block_idx.to_string(),
			self.element_count.to_string(),
			self.child_idx.to_string(),
		]
		.join(",")
	}
	pub fn from_csv(csv: &str) -> ParseResult<Self> {
		let mut parts = csv.split(',');
		let mut next = || -> Result<usize, ParseError> {
			let next = parts
				.next()
				.ok_or_else(|| ParseError::serde("invalid rsx context csv"))?
				.parse()?;
			Ok(next)
		};

		// must keep in sync with to_csv
		let node_idx = next()?;
		let component_idx = next()?;
		let block_idx = next()?;
		let element_count = next()?;
		let child_idx = next()?;

		Ok(Self {
			node_idx,
			component_idx,
			block_idx,
			element_count,
			child_idx,
		})
	}
	/// Depth-first traversal of the tree
	/// identical impl to visit_mut
	pub fn visit(
		node: impl AsRef<RsxNode>,
		mut func: impl FnMut(&Self, &RsxNode),
	) -> Self {
		let mut visitor = Self::default();
		visitor.visit_impl(
			node.as_ref(),
			|cx, node| {
				func(cx, node);
				node
			},
			|queue, node| match node {
				RsxNode::Fragment(rsx_nodes) => {
					for node in rsx_nodes {
						queue.push_back(HtmlElementPosition::MiddleChild(node));
					}
				}
				RsxNode::Component(RsxComponent { node, .. }) => {
					queue.push_back(HtmlElementPosition::MiddleChild(node));
				}
				RsxNode::Block(RsxBlock { initial, .. }) => {
					queue.push_back(HtmlElementPosition::MiddleChild(initial));
				}
				RsxNode::Element(RsxElement { children, .. }) => {
					let num_children = children.len();
					for (i, child) in children.into_iter().enumerate() {
						queue.push_back(HtmlElementPosition::new_child(
							num_children,
							i,
							child,
						));
					}
				}
				RsxNode::Text(_) => {}
				RsxNode::Comment(_) => {}
				RsxNode::Doctype => {}
			},
		);
		visitor
	}

	/// Breadth-first traversal of the rsx tree
	/// identical impl to visit
	pub fn visit_mut(
		mut node: impl AsMut<RsxNode>,
		mut func: impl FnMut(&Self, &mut RsxNode),
	) -> Self {
		let mut visitor = Self::default();
		visitor.visit_impl(
			node.as_mut(),
			|cx, node| {
				func(cx, node);
				node
			},
			|queue, node| match node {
				RsxNode::Fragment(rsx_nodes) => {
					for node in rsx_nodes {
						queue.push_back(HtmlElementPosition::MiddleChild(node));
					}
				}
				RsxNode::Component(RsxComponent { node, .. }) => {
					queue.push_back(HtmlElementPosition::MiddleChild(node));
				}
				RsxNode::Block(RsxBlock { initial, .. }) => {
					queue.push_back(HtmlElementPosition::MiddleChild(initial));
				}
				RsxNode::Element(RsxElement { children, .. }) => {
					let num_children = children.len();
					for (i, child) in children.into_iter().enumerate() {
						queue.push_back(HtmlElementPosition::new_child(
							num_children,
							i,
							child,
						));
					}
				}
				RsxNode::Text(_) => {}
				RsxNode::Comment(_) => {}
				RsxNode::Doctype => {}
			},
		);
		visitor
	}
	/// Breadth-first traversal of the rsx tree
	fn visit_impl<'a, T: Borrow<RsxNode>>(
		&mut self,
		node: T,
		mut func: impl FnMut(&mut Self, T) -> T,
		mut map_children: impl FnMut(&mut VecDeque<HtmlElementPosition<T>>, T),
	) {
		let mut queue = VecDeque::new();
		queue.push_back(HtmlElementPosition::OnlyChild(node));


		while let Some(pos_node) = queue.pop_front() {
			let pos_disc = pos_node.discriminant();
			let node = pos_node.into_inner();
			let node_disc = node.borrow().discriminant();
			self.before_visit_node(&node_disc, &pos_disc);
			// let num_children = node.borrow().children().len();
			let node = func(self, node);
			self.after_visit_node(&node_disc, &pos_disc);
			map_children(&mut queue, node);
		}
	}
}

#[derive(EnumDiscriminants)]
enum HtmlElementPosition<T> {
	OnlyChild(T),
	FirstChild(T),
	MiddleChild(T),
	LastChild(T),
}

impl<T> HtmlElementPosition<T> {
	fn new_child(num_children: usize, i: usize, child: T) -> Self {
		if num_children == 1 {
			HtmlElementPosition::OnlyChild(child)
		} else if i == 0 {
			HtmlElementPosition::FirstChild(child)
		} else if i == num_children - 1 {
			HtmlElementPosition::LastChild(child)
		} else {
			HtmlElementPosition::MiddleChild(child)
		}
	}


	fn discriminant(&self) -> HtmlElementPositionDiscriminants { self.into() }
	pub fn into_inner(self) -> T {
		match self {
			Self::OnlyChild(val)
			| Self::FirstChild(val)
			| Self::MiddleChild(val)
			| Self::LastChild(val) => val,
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;


	#[test]
	fn csv() {
		let a = RsxContext {
			block_idx: 1,
			component_idx: 2,
			element_count: 2,
			child_idx: 3,
			node_idx: 4,
		};
		let csv = a.to_csv();
		let b = RsxContext::from_csv(&csv).unwrap();
		expect(a).to_be(b);
	}

	struct Child;
	impl Component for Child {
		fn render(self) -> RsxRoot {
			rsx! { <div>{8}</div> }
		}
	}

	#[test]
	fn component_idx() {
		expect(
			RsxContext::visit(&rsx! { <div></div> }, |_, _| {}).component_idx,
		)
		.to_be(0);
		expect(RsxContext::visit(&rsx! { <Child /> }, |_, _| {}).component_idx)
			.to_be(1);
	}
	#[test]
	fn rust_blocks() {
		expect(RsxContext::visit(&rsx! { <div></div> }, |_, _| {}).block_idx)
			.to_be(0);
		expect(
			RsxContext::visit(
				&rsx! {
					{7}
					{8}
					{9}
					<Child />
				},
				|_, _| {},
			)
			.block_idx,
		)
		.to_be(4);
	}

	#[test]
	fn element_count() {
		expect(
			RsxContext::visit(&rsx! { <div></div> }, |_, _| {}).element_count,
		)
		.to_be(0);

		expect(
			RsxContext::visit(&rsx! { <div>738</div> }, |_, _| {})
				.element_count,
		)
		.to_be(0);
		expect(
			RsxContext::visit(
				&rsx! {
					<div>
						<b>pow</b>
					</div>
					<Child />
				},
				|_, _| {},
			)
			.element_count,
		)
		.to_be(0);
	}

	#[test]
	fn element_ids() {
		let bucket = mock_bucket();
		let bucket2 = bucket.clone();

		RsxContext::visit(
			&rsx! {
				// 0
				<main>
					// 1
					<article>
						// 3
						<h1>hello world</h1>
					</article>
					// 2
					<article>
						// 4
						<h1>hello world</h1>
					</article>
				</main>
			},
			|cx, node| {
				if matches!(node, RsxNode::Element(_)) {
					bucket2.call(cx.clone());
				}
			},
		);


		expect(&bucket).to_have_been_called_times(5);
		expect(&bucket).to_have_returned_nth_with(0, &RsxContext {
			node_idx: 0,
			component_idx: 0,
			block_idx: 0,
			element_count: 1,
			child_idx: 0,
		});
		expect(&bucket).to_have_returned_nth_with(1, &RsxContext {
			node_idx: 1,
			component_idx: 0,
			block_idx: 0,
			element_count: 1,
			child_idx: 0,
		});
		expect(&bucket).to_have_returned_nth_with(2, &RsxContext {
			node_idx: 2,
			component_idx: 0,
			block_idx: 0,
			element_count: 2,
			child_idx: 1,
		});
		expect(&bucket).to_have_returned_nth_with(3, &RsxContext {
			node_idx: 3,
			component_idx: 0,
			block_idx: 0,
			element_count: 2,
			child_idx: 0,
		});
		expect(&bucket).to_have_returned_nth_with(4, &RsxContext {
			node_idx: 4,
			component_idx: 0,
			block_idx: 0,
			element_count: 2,
			child_idx: 0,
		});
	}
}
