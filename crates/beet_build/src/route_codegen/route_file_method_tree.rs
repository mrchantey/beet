use crate::prelude::*;
use beet_net::prelude::*;
use beet_utils::prelude::VecExt;
use beet_utils::utils::Tree;
use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct RouteFileMethodTree {
	/// The route path for this part of the tree. It may be
	/// a parent or leaf node.
	pub name: PathSegment,
	/// A list of entities with a [`RouteFileMethod`] component
	/// that are associated with this route. These usually
	/// originate from a single file but may come from sepearate collections
	/// if they share the same route path.
	pub funcs: Vec<Entity>,
	/// Children mapped by their [`RouteTreeBuilder::name`].
	/// If this is empty then the route is a leaf node.
	pub children: Vec<RouteFileMethodTree>,
}

impl RouteFileMethodTree {
	pub fn new(segment: impl Into<PathSegment>) -> Self {
		Self {
			name: segment.into(),
			funcs: Vec::new(),
			children: Vec::new(),
		}
	}

	/// usually for debugging, just output all paths
	/// and the route names, collapsing methods with the same path.
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

	/// Returns true if all children of this node have no children
	pub fn all_children_are_leaf_nodes(&self) -> bool {
		self.children.iter().all(|child| child.children.is_empty())
	}

	pub fn flatten(self) -> Vec<Entity> {
		let mut out = Vec::new();
		out.extend(self.funcs.into_iter());
		for child in self.children.into_iter() {
			out.extend(child.flatten());
		}
		out
	}

	pub fn from_methods(funcs: Vec<(Entity, &RouteFileMethod)>) -> Self {
		let mut this = RouteFileMethodTree::new("routes");
		for func in funcs {
			let mut current = &mut this;
			for component in func.1.route_info.path.components() {
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
	use beet_core::prelude::WorldMutExt;
	use beet_net::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;


	fn world() -> World {
		let mut world = World::new();
		world.spawn_batch(vec![
			RouteFileMethod::new("/"),
			RouteFileMethod::new("/bazz"),
			RouteFileMethod::new("/foo/bar"),
			RouteFileMethod::new("/foo/bazz"),
			RouteFileMethod::new("/foo/bazz/boo"),
			RouteFileMethod::new(RouteInfo::post("/foo/bazz/boo")),
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
			.xpect()
			.to_be(
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
