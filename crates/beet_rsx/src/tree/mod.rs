mod tree_iter;
mod tree_iter_flat;
mod tree_map;
mod tree_position;
mod tree_visitor;
use anyhow::Result;
pub use tree_iter::*;
pub use tree_iter_flat::*;
pub use tree_position::*;
pub use tree_visitor::*;

pub trait Tree: Sized {
	type Item;
	fn new_with_value_and_children(
		value: Self::Item,
		children: Vec<Self>,
	) -> Self;
	fn children(&self) -> &[Self];
	fn children_mut(&mut self) -> &mut [Self];
	fn into_children(self) -> Vec<Self>;
	fn value(&self) -> &Self::Item;
	fn value_mut(&mut self) -> &mut Self::Item;
	fn split(&self) -> (&Self::Item, &[Self]);
	fn split_mut(&mut self) -> (&mut Self::Item, &mut [Self]);
	fn split_owned(self) -> (Self::Item, Vec<Self>);
}

// impl<T:Tree> From<(T::Item, Vec<T>)> for T{

// }

// impl<T> Into<TreeStruct<T>> for (T, Vec<T>) {
// 	fn into(self) -> T { T::new_with_value_and_children(self.0, self.1) }
// }





/// A simple tree structure
#[derive(Debug, Clone, PartialEq)]
pub struct Node<T> {
	value: T,
	children: Vec<Node<T>>,
}


impl<T> Node<T> {
	pub fn new(value: T) -> Self {
		Self {
			children: Default::default(),
			value,
		}
	}
	pub fn with_children(mut self, children: Vec<Node<T>>) -> Self {
		self.children = children;
		self
	}

	pub fn with_child(mut self, child: Node<T>) -> Self {
		self.children.push(child);
		self
	}

	/// Attempt to collect a tree from a list of items with positions
	///
	/// # Errors
	/// - if the iterator is empty
	/// - if there is no root
	/// - if a child is visited before its parent
	pub fn collect(
		items: impl IntoIterator<Item = (TreePosition, T)>,
	) -> Result<Self> {
		let mut root = None;

		for (pos, value) in items {
			let path = pos.path();

			if path.is_empty() {
				anyhow::bail!(
					"found empty path for position at index {}",
					pos.index()
				);
			} else if path.len() == 1 {
				if path[0] != 0 {
					anyhow::bail!(
						"found non-zero index for root position at index {}",
						pos.index()
					);
				}
				root = Some(Self::new(value));
				continue;
			} else {
				let root_node = root
					.as_mut()
					.ok_or_else(|| anyhow::anyhow!("no root found"))?;
				let mut current = root_node;

				// Follow path except last index
				for &index in path[..path.len() - 1].iter() {
					current = &mut current.children[index];
				}

				// Handle last index
				let last_idx = *path
					.last()
					.ok_or_else(|| anyhow::anyhow!("no last index found"))?;
				if last_idx != current.children.len() {
					anyhow::bail!(
						"found non-sequential index {} for position at index {}",
						last_idx,
						pos.index()
					);
				}
				current.children.push(Self::new(value));
			}
		}
		Ok(root.ok_or_else(|| anyhow::anyhow!("no root found"))?)
	}
}


impl<T> Tree for Node<T> {
	type Item = T;
	fn new_with_value_and_children(
		value: Self::Item,
		children: Vec<Self>,
	) -> Self {
		Self { value, children }
	}
	fn children(&self) -> &[Self] { &self.children }
	fn children_mut(&mut self) -> &mut [Self] { &mut self.children }
	fn into_children(self) -> Vec<Self> { self.children }
	fn value(&self) -> &Self::Item { &self.value }
	fn value_mut(&mut self) -> &mut Self::Item { &mut self.value }

	fn split(&self) -> (&Self::Item, &[Self]) { (&self.value, &self.children) }
	fn split_mut(&mut self) -> (&mut Self::Item, &mut [Self]) {
		(&mut self.value, &mut self.children)
	}
	fn split_owned(self) -> (Self::Item, Vec<Self>) {
		(self.value, self.children)
	}
}


impl<T> From<T> for Node<T> {
	fn from(value: T) -> Self { Self::new(value) }
}
impl<T, U: Into<Node<T>>> From<(T, Vec<U>)> for Node<T> {
	fn from((value, children): (T, Vec<U>)) -> Self {
		Self::new_with_value_and_children(
			value,
			children.into_iter().map(|c| c.into()).collect(),
		)
	}
}

impl<T: Tree> FromIterator<(TreePosition, T)> for Node<T> {
	fn from_iter<I: IntoIterator<Item = (TreePosition, T)>>(iter: I) -> Self {
		Node::collect(iter).unwrap()
	}
}


#[cfg(test)]
mod test {
	// use super::tree_iter::TreeIter;
	// use crate::prelude::*;
	// use ::sweet::prelude::*;

	// fn create_tree() -> Node<i32> {
	// 	Node::new(1).with_children(vec![
	// 		Node::new(2).with_children(vec![Node::new(3), Node::new(4)]),
	// 		Node::new(5).with_children(vec![Node::new(6)]),
	// 	])
	// }

	#[test]
	fn from_tuple() {
		// let tree = create_tree()
		// 	.iter_dfs()
		// 	.map(|(val, _)| *val)
		// 	.collect::<Node<i32>>();
		// expect(&tree).to_be(&create_tree());
		// // println!("{:?}", tree);

		// expect(tree.into_iter_flat_dfs().collect::<Vec<_>>())
		// 	.to_be(vec![1, 2, 3, 4, 5, 6]);

		// let tree:(i32,Vec< = (1, vec![
		// 	(2, vec![(3, vec![]), (4, vec![])]),
		// 	(5, vec![(6, vec![])]),
		// ]);
		// let tree = Into::<Node<i32>>::into(tree);
	}
}
