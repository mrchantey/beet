use super::Tree;
use std::collections::VecDeque;


pub trait TreeIterPosition: Tree {
	fn iter_with_positions_bfs(&self) -> TreeIterPositionBfs<Self> {
		TreeIterPositionBfs::new(self)
	}
}
impl<T: Tree> TreeIterPosition for T {}


/// Breadth-first iterator over a tree
pub struct TreeIterPositionBfs<'a, T: Tree> {
	queue: VecDeque<(TreePosition, &'a T)>,
}

impl<'a, T: Tree> TreeIterPositionBfs<'a, T> {
	pub fn new(tree: &'a T) -> Self {
		let mut queue = VecDeque::with_capacity(1);
		queue.push_back((TreePosition::root(), tree));
		Self { queue }
	}
}

impl<'a, T: Tree> Iterator for TreeIterPositionBfs<'a, T> {
	type Item = (TreePosition, &'a T);

	fn next(&mut self) -> Option<Self::Item> {
		if let Some((position, val)) = self.queue.pop_front() {
			let mut child_positions = position.clone();
			child_positions.push_child();

			let children = val
				.children()
				.into_iter()
				.map(|c| {
					let child_pos = child_positions.clone();
					child_positions.next_sibling();
					(child_pos, c)
				})
				.collect::<Vec<_>>();

			self.queue.extend(children);
			Some((position, val))
		} else {
			None
		}
	}
}




/// Track the position of items in a tree,
/// these methods should be 'outer' to any visitors,
/// ie call `visit_node` before visiting children, and `leave_node` after
/// and call `visit_children` before visiting children, and `leave_children` after
///
/// Represents the position of a node in the tree.
/// This always has at least one element.
///
/// Considering the following:
/// ```html
/// <html data-sweet-pos="0">
/// 	<head data-sweet-pos="0,0"></head>
/// 	<body data-sweet-pos="0,1">
/// 		<div data-sweet-pos="0,1,0"></div>
/// 		<div data-sweet-pos="0,1,1"></div>
/// 	</body>
/// </html>
/// ```
///
///
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreePosition {
	/// Vec of child indices, ie:
	/// [0,2,1] means: first child -> third child -> second child
	path: Vec<usize>,
	/// a number that increments when a node is visited,
	/// this will be '1' when the first child is visited,
	/// so we use current_index() - 1 to get the current index
	node_count: usize,
}

impl TreePosition {
	/// Create a position with a single node
	pub fn root() -> Self {
		Self {
			path: Vec::from([0]),
			node_count: 1,
		}
	}

	/// An inverse of the path used to map fro a root to the position
	pub fn breadcrumbs(&self) -> Vec<usize> {
		let mut path = self.path.clone();
		path.reverse();
		path
	}

	pub fn path(&self) -> &Vec<usize> { &self.path }
	/// The node count - 1
	/// # Panics
	/// If no nodes have been visited
	pub fn index(&self) -> usize { self.node_count - 1 }
	pub fn node_count(&self) -> usize { self.node_count }

	pub fn next_sibling(&mut self) {
		*self.path.last_mut().expect("tree is empty") += 1;
		self.node_count += 1;
	}
	/// `path.last++, index--`
	/// # Panics
	/// if there are no positions
	pub fn prev_sibling(&mut self) {
		*self.path.last_mut().expect("tree is empty") -= 1;
		self.node_count -= 1;
	}
	/// `path.push(0)`
	pub fn push_child(&mut self) {
		self.path.push(0);
		self.node_count += 1;
	}
	/// `path.pop()`
	/// # Panics
	/// - if there are no positions
	/// - if the last child index causes node count to be negative
	pub fn pop_child(&mut self) {
		let num_children = self.path.pop().unwrap() + 1;
		self.node_count -= num_children;
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	fn create_tree() -> TreeNode2<i32> {
		TreeNode2::new(0).with_children(vec![
			TreeNode2::new(1).with_child(TreeNode2::new(4)),
			TreeNode2::new(2),
			TreeNode2::new(3),
		])
	}


	#[test]
	fn works() {
		let tree = create_tree();
		let _items = tree
			.iter_with_positions_bfs()
			.map(|(pos, _)| pos)
			.collect::<Vec<_>>();

		// expect(&items[0]).to_be(TreePosition:);
		// expect(&items[1]).to_be("2,0,0:1");
		// expect(&items[2]).to_be("3,0,1:2");
		// expect(&items[3]).to_be("4,0,2:3");
		// expect(&items[4]).to_be("3,0,0,0:4");
	}


	#[test]
	fn tree_position() {
		/*
		Simulate the following tree
		```
		p0
			p1
			p2
				p3
		p4
		```
		*/
		let p0 = TreePosition {
			node_count: 1,
			path: vec![0],
		};
		let p1 = TreePosition {
			node_count: 2,
			path: vec![0, 0],
		};
		let p2 = TreePosition {
			node_count: 3,
			path: vec![0, 1],
		};
		let p3 = TreePosition {
			node_count: 4,
			path: vec![0, 1, 0],
		};

		let p4 = TreePosition {
			node_count: 3,
			path: vec![0, 1],
		};


		let mut pos = TreePosition::default();
		pos.push_child(); // go to 0
		expect(&pos).to_be(&p0);
		pos.push_child(); // go to 1
		expect(&pos).to_be(&p1);
		pos.next_sibling(); // go to 2
		expect(&pos).to_be(&p2);
		pos.prev_sibling(); // go to 1
		expect(&pos).to_be(&p1);
		pos.next_sibling(); // go to 2
		expect(&pos).to_be(&p2);
		pos.push_child(); // go to 3
		expect(&pos).to_be(&p3);
		pos.pop_child(); // go to 2
		expect(&pos).to_be(&p4);
	}
}
