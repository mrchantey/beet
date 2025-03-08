use super::Tree;
use anyhow::Result;
use anyhow::anyhow;
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

	/// Convert to a comma separated value string, with the first index
	/// representing the **node count**, not index.
	/// ie "1,1,2"
	pub fn to_csv(&self) -> String {
		let mut values = vec![self.node_count.to_string()];
		values.extend(self.path.iter().map(|i| i.to_string()));
		values.join(",")
	}

	/// Tree position from comma separated values, ie "1,1,2"
	/// # Errors
	/// - if not empty and node count is zero
	/// - if any of the values are not parsable as usize
	pub fn from_csv(csv: &str) -> anyhow::Result<Self> {
		let values: Vec<usize> = csv
			.split(",")
			.map(|s| {
				s.parse().map_err(|e| {
					anyhow!("failed to parse csv for TreePosition: {s}\n{}", e)
				})
			})
			.collect::<Result<Vec<_>>>()?;

		if values.is_empty() {
			return Ok(Self {
				path: vec![],
				node_count: 0,
			});
		}
		if values[0] == 0 {
			return Err(anyhow!("node count cannot be zero"));
		}

		Ok(Self {
			node_count: values[0],
			path: values[1..].to_vec(),
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	fn create_tree() -> Node<i32> {
		Node::new(0).with_children(vec![
			Node::new(1).with_child(Node::new(4)),
			Node::new(2),
			Node::new(3),
		])
	}


	#[test]
	fn works() {
		// let a = vec![3].enu

		let tree = create_tree();
		let items = tree
			.iter_with_positions_bfs()
			.map(|(pos, val)| format!("{}:{}", pos.to_csv(), val.value))
			.collect::<Vec<_>>();

		expect(&items[0]).to_be("1,0:0");
		expect(&items[1]).to_be("2,0,0:1");
		expect(&items[2]).to_be("3,0,1:2");
		expect(&items[3]).to_be("4,0,2:3");
		expect(&items[4]).to_be("3,0,0,0:4");

		// let node = create_tree()
		// 	.iter_with_positions_bfs()
		// 	.collect::<Node<&i32>>();
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
