use super::Tree;
use std::collections::VecDeque;

pub enum LeafOrBranch<T> {
	Leaf(T),
	Branch(T, Vec<LeafOrBranch<T>>),
}

pub trait TreeIter: Tree {
	fn iter_bfs(&self) -> TreeIterBfs<Self> { TreeIterBfs::new(self) }
	fn iter_dfs(&self) -> TreeIterDfs<Self> { TreeIterDfs::new(self) }
	// fn iter_mut_bfs(&mut self) -> TreeIterMutBfs<Self> {
	// 	TreeIterMutBfs::new(self)
	// }
	// fn iter_mut_dfs(&mut self) -> TreeIterMutDfs<Self> {
	// 	TreeIterMutDfs::new(self)
	// }
	// fn into_iter_bfs(self) -> TreeIntoIterBfs<Self> {
	// 	TreeIntoIterBfs::new(self)
	// }
	// fn into_iter_dfs(self) -> TreeIntoIterDfs<Self> {
	// 	TreeIntoIterDfs::new(self)
	// }
}
impl<T: Tree> TreeIter for T {}

/// Breadth-first iterator over a tree
pub struct TreeIterBfs<'a, T: Tree> {
	queue: VecDeque<&'a T>,
}

impl<'a, T: Tree> TreeIterBfs<'a, T> {
	pub fn new(tree: &'a T) -> Self {
		let mut queue = VecDeque::with_capacity(1);
		queue.push_back(tree);
		Self { queue }
	}
}

impl<'a, T: Tree> Iterator for TreeIterBfs<'a, T> {
	type Item = &'a T;

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(val) = self.queue.pop_front() {
			self.queue.extend(val.children());
			Some(val)
		} else {
			None
		}
	}
}

/// Depth-first iterator over a tree
pub struct TreeIterDfs<'a, T: Tree> {
	stack: Vec<(&'a T::Item, &'a [T])>,
}

impl<'a, T: Tree> TreeIterDfs<'a, T> {
	pub fn new(tree: &'a T) -> Self {
		let mut stack = Vec::with_capacity(1);
		stack.push(tree.split());
		Self { stack }
	}
}

impl<'a, T: Tree> Iterator for TreeIterDfs<'a, T> {
	type Item = (&'a T::Item, &'a [T]);

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(val) = self.stack.pop() {
			self.stack
				.extend(val.1.iter().rev().map(|child| child.split()));
			Some(val)
		} else {
			None
		}
	}
}

// /// Breadth-first mutable iterator over a tree
// pub struct TreeIterMutBfs<'a, T: Tree> {
// 	queue: VecDeque<(&'a mut T::Item, &'a mut [T])>,
// }

// impl<'a, T: Tree> TreeIterMutBfs<'a, T> {
// 	pub fn new(tree: &'a mut T) -> Self {
// 		let mut queue = VecDeque::with_capacity(1);
// 		queue.push_back(tree.split_mut());
// 		Self { queue }
// 	}
// }

// impl<'a, T: Tree> Iterator for TreeIterMutBfs<'a, T> {
// 	type Item = (&'a mut T::Item, &'a mut [T]);

// 	fn next(&mut self) -> Option<Self::Item> {
// 		if let Some(val) = self.queue.pop_front() {
// 			self.queue
// 				.extend(val.1.iter_mut().map(|child| child.split_mut()));
// 			None
// 			// Some(val)
// 		} else {
// 			None
// 		}
// 	}
// }

// /// Depth-first mutable iterator over a tree
// pub struct TreeIterMutDfs<'a, T: Tree> {
// 	stack: Vec<(&'a mut T::Item, &'a mut [T])>,
// }

// impl<'a, T: Tree> TreeIterMutDfs<'a, T> {
// 	pub fn new(tree: &'a mut T) -> Self {
// 		let mut stack = Vec::with_capacity(1);
// 		stack.push(tree.split_mut());
// 		Self { stack }
// 	}
// }

// impl<'a, T: Tree> Iterator for TreeIterMutDfs<'a, T> {
// 	type Item = (&'a mut T::Item, &'a mut [T]);

// 	fn next(&mut self) -> Option<Self::Item> {
// 		if let Some(val) = self.stack.pop() {
// 			self.stack
// 				.extend(val.1.iter_mut().rev().map(|child| child.split_mut()));
// 			Some(val)
// 		} else {
// 			None
// 		}
// 	}
// }

// /// Breadth-first owned iterator over a tree
// pub struct TreeIntoIterBfs<T: Tree> {
// 	queue: VecDeque<(T::Item, Vec<T>)>,
// }

// impl<T: Tree> TreeIntoIterBfs<T> {
// 	pub fn new(tree: T) -> Self {
// 		let mut queue = VecDeque::with_capacity(1);
// 		queue.push_back(tree.split_owned());
// 		Self { queue }
// 	}
// }

// impl<T: Tree> Iterator for TreeIntoIterBfs<T> {
// 	type Item = (T::Item, Vec<T>);

// 	fn next(&mut self) -> Option<Self::Item> {
// 		if let Some(val) = self.queue.pop_front() {
// 			self.queue
// 				.extend(val.1.into_iter().map(|child| child.split_owned()));
// 			Some(val)
// 		} else {
// 			None
// 		}
// 	}
// }

// /// Depth-first owned iterator over a tree
// pub struct TreeIntoIterDfs<T: Tree> {
// 	stack: Vec<(T::Item, Vec<T>)>,
// }

// impl<T: Tree> TreeIntoIterDfs<T> {
// 	pub fn new(tree: T) -> Self {
// 		let mut stack = Vec::with_capacity(1);
// 		stack.push(tree.split_owned());
// 		Self { stack }
// 	}
// }

// impl<T: Tree> Iterator for TreeIntoIterDfs<T> {
// 	type Item = (T::Item, Vec<T>);

// 	fn next(&mut self) -> Option<Self::Item> {
// 		if let Some(val) = self.stack.pop() {
// 			self.stack.extend(
// 				val.1.into_iter().rev().map(|child| child.split_owned()),
// 			);
// 			Some(val)
// 		} else {
// 			None
// 		}
// 	}
// }
#[cfg(test)]
mod test {
	use super::TreeIter;
	use crate::prelude::*;
	// use sweet::prelude::*;


	/// create a tree labeled in dfs order
	fn tree() -> TreeNode<i32> {
		TreeNode::new(0).with_children(vec![
			TreeNode::new(1).with_children(vec![TreeNode::new(2)]),
			TreeNode::new(3),
			TreeNode::new(4),
		])
	}

	#[test]
	fn works() {
		let _a = vec![32];
		// a.iter().collect();

		let tree = tree().iter_bfs().map(|val| val.value).collect::<Vec<_>>();
		assert_eq!(tree, vec![0, 1, 3, 4, 2]);
	}
}
