use crate::prelude::*;
use beet_utils::prelude::VecExt;
use beet_utils::utils::Tree;
use syn::Item;

#[derive(Debug, Clone)]
pub struct RouteFileMethodTree {
	/// The route path for this part of the tree. It may be
	/// a parent or leaf node.
	pub name: String,
	pub funcs: Vec<RouteFileMethod>,
	/// Children mapped by their [`RouteTreeBuilder::name`].
	/// If this is empty then the route is a leaf node.
	pub children: Vec<RouteFileMethodTree>,
}

impl RouteFileMethodTree {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			funcs: Vec::new(),
			children: Vec::new(),
		}
	}

	/// usually for debugging, just output all paths
	/// and the route names, collapsing methods with the same path.
	#[allow(dead_code)]
	pub fn into_path_tree(&self) -> Tree<String> {
		let mut children = self
			.children
			.iter()
			.map(|child| child.into_path_tree())
			.collect::<Vec<_>>();

		children.sort_by(|a, b| a.value.cmp(&b.value));
		Tree {
			value: self.name.clone(),
			children,
		}
	}


	/// Create a tree of `mod` items, mapping each leaf node.
	pub fn mod_tree(&self, map_item: impl Fn(&Self) -> Item + Clone) -> Item {
		if self.children.is_empty() {
			map_item(self)
		} else {
			let children = self
				.children
				.iter()
				.map(|child| child.mod_tree(map_item.clone()));
			let ident =
				syn::Ident::new(&self.name, proc_macro2::Span::call_site());
			let item = map_item(self);
			syn::parse_quote!(
				#[allow(missing_docs)]
				pub mod #ident {
					#[allow(unused_imports)]
					use super::*;
					#item
					#(#children)*
				}
			)
		}
	}

	/// Returns true if all children of this node have no children
	pub fn all_children_are_leaf_nodes(&self) -> bool {
		self.children.iter().all(|child| child.children.is_empty())
	}

	pub fn flatten(self) -> Vec<RouteFileMethod> {
		let mut out = Vec::new();
		out.extend(self.funcs.into_iter());
		for child in self.children.into_iter() {
			out.extend(child.flatten());
		}
		out
	}

	pub fn from_methods(funcs: Vec<RouteFileMethod>) -> Self {
		let mut this = RouteFileMethodTree::new("root");
		for func in funcs {
			let mut current = &mut this;
			for component in func.route_info.path.components() {
				match component {
					std::path::Component::Normal(os_str)
						if let Some(str) = os_str.to_str() =>
					{
						current = VecExt::entry_or_insert_with(
							&mut current.children,
							|child| child.name == str,
							|| RouteFileMethodTree::new(str),
						);
					}
					_ => {}
				}
			}
			current.funcs.push(func);
		}
		this
	}
}

impl Into<RouteFileMethodTree> for Vec<RouteFileMethod> {
	fn into(self) -> RouteFileMethodTree {
		RouteFileMethodTree::from_methods(self)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	fn tree() -> RouteFileMethodTree {
		vec![
			RouteFileMethod::new("/"),
			RouteFileMethod::new("/bazz"),
			RouteFileMethod::new("/foo/bar"),
			RouteFileMethod::new("/foo/bazz"),
			RouteFileMethod::new("/foo/bazz/boo"),
			RouteFileMethod::new(RouteInfo::post("/foo/bazz/boo")),
		]
		.into()
	}

	#[test]
	fn correct_tree_structure() {
		expect(tree().into_path_tree().to_string_indented()).to_be(
			r#"root
  bazz
  foo
    bar
    bazz
      boo
"#,
		);
	}
}
