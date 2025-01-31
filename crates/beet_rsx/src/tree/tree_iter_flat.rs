use super::Tree;
use std::collections::VecDeque;

pub trait TreeIterFlat: Tree {
	/// breadth-first iterator over all items of in a tree
	fn iter_flat_bfs(&self) -> TreeIterFlatBfs<Self> {
		TreeIterFlatBfs::new(self)
	}
	/// depth-first iterator over all items of in a tree
	fn iter_flat_dfs(&self) -> TreeIterFlatDfs<Self> {
		TreeIterFlatDfs::new(self)
	}
	/// breadth-first mutable iterator over all items of in a tree
	fn iter_flat_mut_bfs(&mut self) -> TreeIterFlatMutBfs<Self> {
		TreeIterFlatMutBfs::new(self)
	}
	/// depth-first mutable iterator over all items of in a tree
	fn iter_flat_mut_dfs(&mut self) -> TreeIterFlatMutDfs<Self> {
		TreeIterFlatMutDfs::new(self)
	}
	/// breadth-first owned iterator over all items of in a tree
	fn into_iter_flat_bfs(self) -> TreeIntoIterFlatBfs<Self> {
		TreeIntoIterFlatBfs::new(self)
	}
	/// depth-first owned iterator over all items of in a tree
	fn into_iter_flat_dfs(self) -> TreeIntoIterFlatDfs<Self> {
		TreeIntoIterFlatDfs::new(self)
	}
}
impl<T: Tree> TreeIterFlat for T {}


pub struct TreeIterFlatBfs<'a, T> {
	queue: VecDeque<&'a T>,
}

impl<'a, T> TreeIterFlatBfs<'a, T> {
	pub fn new(tree: &'a T) -> Self {
		let mut queue = VecDeque::with_capacity(1);
		queue.push_back(tree);
		Self { queue }
	}
}

impl<'a, T: Tree> Iterator for TreeIterFlatBfs<'a, T> {
	type Item = &'a T::Item;

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(node) = self.queue.pop_front() {
			self.queue.extend(node.children());
			Some(node.value())
		} else {
			None
		}
	}
}

/// Depth-first iterator over a tree
pub struct TreeIterFlatDfs<'a, T> {
	stack: Vec<&'a T>,
}

impl<'a, T> TreeIterFlatDfs<'a, T> {
	pub fn new(tree: &'a T) -> Self {
		let mut stack = Vec::with_capacity(1);
		stack.push(tree);
		Self { stack }
	}
}

impl<'a, T: Tree> Iterator for TreeIterFlatDfs<'a, T> {
	type Item = &'a T::Item;

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(node) = self.stack.pop() {
			// Add children in reverse order so leftmost is popped first
			self.stack.extend(node.children().iter().rev());
			Some(node.value())
		} else {
			None
		}
	}
}


/// Breadth-first mutable iterator over a tree
pub struct TreeIterFlatMutBfs<'a, T> {
	queue: VecDeque<&'a mut T>,
}

impl<'a, T> TreeIterFlatMutBfs<'a, T> {
	pub fn new(tree: &'a mut T) -> Self {
		let mut queue = VecDeque::with_capacity(1);
		queue.push_back(tree);
		Self { queue }
	}
}

impl<'a, T: Tree> Iterator for TreeIterFlatMutBfs<'a, T> {
	type Item = &'a mut T::Item;

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(node) = self.queue.pop_front() {
			let (value, children) = node.split_mut();
			self.queue.extend(children);
			Some(value)
		} else {
			None
		}
	}
}

/// Depth-first mutable iterator over a tree
pub struct TreeIterFlatMutDfs<'a, T> {
	stack: Vec<&'a mut T>,
}

impl<'a, T> TreeIterFlatMutDfs<'a, T> {
	pub fn new(tree: &'a mut T) -> Self {
		let mut stack = Vec::with_capacity(1);
		stack.push(tree);
		Self { stack }
	}
}

impl<'a, T: Tree> Iterator for TreeIterFlatMutDfs<'a, T> {
	type Item = &'a mut T::Item;

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(node) = self.stack.pop() {
			let (value, children) = node.split_mut();
			// Add children in reverse order so leftmost is popped first
			self.stack.extend(children.into_iter().rev());
			Some(value)
		} else {
			None
		}
	}
}


/// Breadth-first mutable iterator over a tree
pub struct TreeIntoIterFlatBfs<T> {
	queue: VecDeque<T>,
}

impl<T> TreeIntoIterFlatBfs<T> {
	pub fn new(tree: T) -> Self {
		let mut queue = VecDeque::with_capacity(1);
		queue.push_back(tree);
		Self { queue }
	}
}

impl<T: Tree> Iterator for TreeIntoIterFlatBfs<T> {
	type Item = T::Item;

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(node) = self.queue.pop_front() {
			let (value, children) = node.split_owned();
			self.queue.extend(children);
			Some(value)
		} else {
			None
		}
	}
}

/// Depth-first mutable iterator over a tree
pub struct TreeIntoIterFlatDfs<T> {
	stack: Vec<T>,
}

impl<T> TreeIntoIterFlatDfs<T> {
	pub fn new(tree: T) -> Self {
		let mut stack = Vec::with_capacity(1);
		stack.push(tree);
		Self { stack }
	}
}

impl<T: Tree> Iterator for TreeIntoIterFlatDfs<T> {
	type Item = T::Item;

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(node) = self.stack.pop() {
			let (value, children) = node.split_owned();
			// Add children in reverse order so leftmost is popped first
			self.stack.extend(children.into_iter().rev());
			Some(value)
		} else {
			None
		}
	}
}

#[cfg(test)]
mod test {
	// use crate::prelude::*;
	use super::TreeIterFlat;
	use crate::tree::Node;
	use sweet::prelude::*;


	/// create a tree labeled in dfs order
	fn tree() -> Node<i32> {
		Node::new(0).with_children(vec![
			Node::new(1).with_children(vec![Node::new(2)]),
			Node::new(3),
			Node::new(4),
		])
	}


	#[test]
	fn iter() {
		let foo = tree();
		let items = foo.iter_flat_dfs().map(|x| *x).collect::<Vec<_>>();
		expect(items).to_be(vec![0, 1, 2, 3, 4]);
		let items = foo.iter_flat_bfs().map(|x| *x).collect::<Vec<_>>();
		expect(items).to_be(vec![0, 1, 3, 4, 2]);
	}
	#[test]
	fn iter_mut() {
		let mut foo = tree();
		foo.iter_flat_mut_dfs().for_each(|x| *x += 1);
		foo.iter_flat_mut_bfs().for_each(|x| *x += 1);

		let items = foo.iter_flat_dfs().map(|x| *x).collect::<Vec<_>>();
		expect(items).to_be(vec![2, 3, 4, 5, 6]);
		let items = foo.iter_flat_bfs().map(|x| *x).collect::<Vec<_>>();
		expect(items).to_be(vec![2, 3, 5, 6, 4]);
	}
	#[test]
	fn into_iter() {
		let items = tree().into_iter_flat_dfs().collect::<Vec<_>>();
		expect(items).to_be(vec![0, 1, 2, 3, 4]);
		let items = tree().into_iter_flat_bfs().collect::<Vec<_>>();
		expect(items).to_be(vec![0, 1, 3, 4, 2]);
	}
}
