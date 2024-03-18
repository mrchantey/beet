use crate::prelude::*;
use petgraph::graph::DiGraph;
use serde::ser::SerializeStruct;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;

pub struct Tree<T> {
	pub value: T,
	pub children: Vec<Tree<T>>,
}

impl<T: Serialize> Serialize for Tree<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut state = serializer.serialize_struct("Tree", 2)?;
		state.serialize_field("value", &self.value)?;
		state.serialize_field("children", &self.children)?;
		state.end()
	}
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Tree<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct TreeHelper<T> {
			value: T,
			children: Vec<Tree<T>>,
		}
		let helper = TreeHelper::deserialize(deserializer)?;
		Ok(Self {
			value: helper.value,
			children: helper.children,
		})
	}
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

impl<T: Default> Default for Tree<T> {
	fn default() -> Self {
		Self {
			value: T::default(),
			children: Vec::new(),
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
	pub fn new_with_children(value: T, children: Vec<Self>) -> Self {
		Self { value, children }
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

	pub fn into_graph(self) -> DiGraph<T, ()> { DiGraph::from_tree(self) }

	pub fn map<O>(&self, mut map_func: impl FnMut(&T) -> O) -> Tree<O> {
		self.map_ref(&mut map_func)
	}
	fn map_ref<O>(&self, map_func: &mut impl FnMut(&T) -> O) -> Tree<O> {
		Tree {
			value: map_func(&self.value),
			children: self
				.children
				.iter()
				.map(|child| child.map_ref(map_func))
				.collect(),
		}
	}
	pub fn map_owned<O>(self, mut map_func: impl FnMut(T) -> O) -> Tree<O> {
		self.map_owned_ref(&mut map_func)
	}
	fn map_owned_ref<O>(self, map_func: &mut impl FnMut(T) -> O) -> Tree<O> {
		Tree {
			value: map_func(self.value),
			children: self
				.children
				.into_iter()
				.map(|child| child.map_owned_ref(map_func))
				.collect(),
		}
	}
}


impl<T: Debug> Tree<T> {
	/// Creates a string from this tree, in the format
	/// ```ignore
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
	/// ```ignore
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
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let tree = Tree::new(0).with_leaf(1).with_leaf(2);
		let tree2 = tree.map(|x| x + 1);
		expect(tree2).to_be(Tree::new(1).with_leaf(2).with_leaf(3))?;

		Ok(())
	}
}
