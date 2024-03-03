use crate::prelude::*;
use petgraph::graph::DiGraph;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;

pub struct Tree<T> {
	pub value: T,
	pub children: Vec<Tree<T>>,
}

impl<T: Debug> Debug for Tree<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		if self.children.len() > 0 {
			f.debug_struct("Tree")
				.field("value", &self.value)
				.field("children", &self.children)
				.finish()
		} else {
			f.debug_struct("Tree").field("value", &self.value).finish()
		}
	}
}

impl<T: Display> Display for Tree<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.pretty_string_display(0))
	}
}

impl<T: Clone> Clone for Tree<T> {
	fn clone(&self) -> Self {
		Self {
			value: self.value.clone(),
			children: self.children.clone(),
		}
	}
}

impl<T: PartialEq> PartialEq for Tree<T> {
	fn eq(&self, other: &Self) -> bool {
		self.value == other.value && self.children == other.children
	}
}

impl<T> Tree<T> {
	pub fn new(value: T) -> Self {
		Self {
			value,
			children: Vec::new(),
		}
	}
	/// Add a child tree, which may have children
	pub fn with_child(mut self, child: impl Into<Tree<T>>) -> Self {
		self.children.push(child.into());
		self
	}
	/// Add child trees, which may have children
	pub fn with_children(mut self, children: Vec<Tree<T>>) -> Self {
		self.children.extend(children);
		self
	}
	/// Add a terminating (childless) child node
	pub fn with_leaf(mut self, child: T) -> Self {
		self.children.push(Tree::new(child));
		self
	}
	pub fn new_with_children(value: T, children: Vec<Self>) -> Self {
		Self { value, children }
	}

	pub fn into_graph(self) -> DiGraph<T, ()> { DiGraph::from_tree(self) }
}


impl<T: Debug> Tree<T> {
	/// Creates a string from this tree, in the format
	/// ```
	/// value
	/// 	Child0.value
	/// 		Child0.Child0.value
	/// 	Child1.value
	/// etc.
	/// ```
	pub fn pretty_string_debug(&self, depth: usize) -> String {
		let mut string = String::new();
		string.push_str(&format!(
			"{}{:?}\n",
			String::from_utf8(vec![b'\t'; depth]).unwrap(),
			self.value
		));
		for child in self.children.iter() {
			string.push_str(&child.pretty_string_debug(depth + 1));
		}
		string
	}
}
impl<T: Display> Tree<T> {
	/// Creates a string from this tree, in the format
	/// ```
	/// value
	/// 	Child0.value
	/// 		Child0.Child0.value
	/// 	Child1.value
	/// etc.
	/// ```
	pub fn pretty_string_display(&self, depth: usize) -> String {
		let mut string = String::new();
		string.push_str(&format!(
			"{}{}\n",
			String::from_utf8(vec![b'\t'; depth]).unwrap(),
			self.value
		));
		for child in self.children.iter() {
			string.push_str(&child.pretty_string_display(depth + 1));
		}
		string
	}
}

// pub trait IntoTree<T, M> {
// 	fn into_tree(self) -> Tree<T>;
// }
