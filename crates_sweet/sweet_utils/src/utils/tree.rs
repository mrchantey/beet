use std::fmt;

/// Very simple general purpose tree structure.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tree<T> {
	/// The value of this node.
	pub value: T,
	/// The children of this node.
	pub children: Vec<Tree<T>>,
}
impl<T> Tree<T> {
	pub fn new(value: T) -> Self {
		Self {
			value,
			children: Default::default(),
		}
	}
	pub fn new_with_children(value: T, children: Vec<Tree<T>>) -> Self {
		Self { value, children }
	}
	pub fn with_children(mut self, children: Vec<Tree<T>>) -> Self {
		self.children = children;
		self
	}

	/// Iterates over the children of this node and applies the function to each child,
	/// returning the first child that matches the predicate.
	/// If no child matches, it creates a new child with the default value of T and returns it.
	pub fn find_or_insert<'a>(
		&'a mut self,
		func: impl Fn(&T) -> bool,
	) -> &'a mut Tree<T>
	where
		T: Default,
	{
		for i in 0..self.children.len() {
			if func(&self.children[i].value) {
				return &mut self.children[i];
			}
		}
		
		// If no child matches, create and add a new one
		self.children.push(Tree::new(T::default()));
		self.children.last_mut().unwrap()
	}



	pub fn sort_recursive(&mut self)
	where
		T: Ord,
	{
		self.children.sort_by(|a, b| a.value.cmp(&b.value));
		for child in &mut self.children {
			child.sort_recursive();
		}
	}

	pub fn to_string_indented(&self) -> String
	where
		T: fmt::Display,
	{
		let mut str = String::new();
		self.to_string_indented_inner(&mut str, 0);
		str
	}
	fn to_string_indented_inner(&self, buffer: &mut String, indent: usize)
	where
		T: fmt::Display,
	{
		buffer.push_str(&" ".repeat(indent));
		buffer.push_str(&self.value.to_string());
		buffer.push('\n');
		for child in &self.children {
			child.to_string_indented_inner(buffer, indent + 2);
		}
	}
}

impl<T> From<T> for Tree<T> {
	fn from(value: T) -> Self { Self::new(value) }
}


impl<T: fmt::Display> fmt::Display for Tree<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.children.is_empty() {
			write!(f, "{}", self.value)
		} else {
			let children = self
				.children
				.iter()
				.map(|c| c.to_string())
				.collect::<Vec<_>>()
				.join(", ");
			write!(f, "({}, [{}])", self.value, children)
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn works() {
		let tree = Tree::new("root").with_children(vec![
			Tree::new("child1"),
			Tree::new("child2").with_children(vec![
				Tree::new("grandchild1"),
				Tree::new("grandchild2"),
			]),
		]);
		assert_eq!(
			tree.to_string(),
			"(root, [child1, (child2, [grandchild1, grandchild2])])"
		);
		assert_eq!(
			tree.to_string_indented(),
			r#"root
  child1
  child2
    grandchild1
    grandchild2
"#
		);
		assert_eq!(tree.value, "root");
		assert_eq!(tree.children.len(), 2);
		assert_eq!(tree.children[0].value, "child1");
		assert_eq!(tree.children[1].value, "child2");
		assert_eq!(tree.children[1].children.len(), 2);
		assert_eq!(tree.children[1].children[0].value, "grandchild1");
		assert_eq!(tree.children[1].children[1].value, "grandchild2");
	}
}
