//! Route file method tree structure for organizing routes hierarchically.
//!
//! This module provides a tree data structure for organizing [`RouteFileMethod`]
//! entities by their route paths, enabling hierarchical code generation.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_router::prelude::*;

/// A tree structure organizing [`RouteFileMethod`] entities by route path segments.
///
/// This structure is used to generate hierarchical module structures for client
/// actions, where each path segment becomes a nested module.
#[derive(Debug, Clone)]
pub(crate) struct RouteFileMethodTree {
	/// The route path segment for this part of the tree.
	///
	/// May be a parent or leaf node.
	pub name: PathPatternSegment,
	/// Entities with a [`RouteFileMethod`] component associated with this route.
	///
	/// These usually originate from a single file but may come from separate
	/// collections if they share the same route path.
	pub funcs: Vec<Entity>,
	/// Children mapped by their path segment name.
	///
	/// If this is empty, the route is a leaf node.
	pub children: Vec<RouteFileMethodTree>,
}

#[allow(unused)]
impl RouteFileMethodTree {
	/// Creates a new tree node with the given path segment.
	pub fn new(segment: impl Into<PathPatternSegment>) -> Self {
		Self {
			name: segment.into(),
			funcs: Vec::new(),
			children: Vec::new(),
		}
	}

	/// Converts the tree to a path tree for debugging.
	///
	/// Outputs all paths and route names, collapsing methods with the same path.
	#[allow(dead_code)]
	pub fn into_path_tree(&self) -> Tree<String> {
		let children = self
			.children
			.iter()
			.map(|child| child.into_path_tree())
			.collect::<Vec<_>>();

		Tree {
			value: self.name.to_string(),
			children,
		}
	}

	/// Returns `true` if all children of this node have no children.
	pub fn all_children_are_leaf_nodes(&self) -> bool {
		self.children.iter().all(|child| child.children.is_empty())
	}

	/// Flattens the tree into a vector of all entities.
	pub fn flatten(self) -> Vec<Entity> {
		let mut out = Vec::new();
		out.extend(self.funcs.into_iter());
		for child in self.children.into_iter() {
			out.extend(child.flatten());
		}
		out
	}

	/// Builds a tree from a list of route file methods.
	///
	/// Each method's path is decomposed into segments, and the tree is built
	/// by inserting each method at the appropriate depth.
	pub fn from_methods(funcs: Vec<(Entity, &RouteFileMethod)>) -> Self {
		let mut this = RouteFileMethodTree::new("routes");
		for func in funcs {
			let mut current = &mut this;
			for component in func.1.path.components() {
				match component {
					std::path::Component::Normal(os_str)
						if let Some(str) = os_str.to_str() =>
					{
						current = VecExt::entry_or_insert_with(
							&mut current.children,
							|child| child.name.as_str() == str,
							|| RouteFileMethodTree::new(str),
						);
					}
					_ => {}
				}
			}
			current.funcs.push(func.0);
		}
		this
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;


	fn world() -> World {
		let mut world = World::new();
		world.spawn_batch(vec![
			RouteFileMethod::new("/", HttpMethod::Get),
			RouteFileMethod::new("/bazz", HttpMethod::Get),
			RouteFileMethod::new("/foo/bar", HttpMethod::Get),
			RouteFileMethod::new("/foo/bazz", HttpMethod::Get),
			RouteFileMethod::new("/foo/bazz/boo", HttpMethod::Get),
			RouteFileMethod::new("/foo/bazz/boo", HttpMethod::Post),
		]);
		world
	}

	#[test]
	fn correct_tree_structure() {
		let mut world = world();
		let methods = world
			.query_once::<(Entity, &RouteFileMethod)>()
			.iter()
			.copied()
			.collect();
		RouteFileMethodTree::from_methods(methods)
			.into_path_tree()
			.to_string_indented()
			.xpect_eq(
				r#"routes
  bazz
  foo
    bar
    bazz
      boo
"#,
			);
	}
}
