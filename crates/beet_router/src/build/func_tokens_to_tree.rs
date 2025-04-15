use crate::prelude::*;
use sweet::prelude::*;
use syn::Item;


#[derive(Debug, Default, Clone)]
pub struct FuncTokensToTree;



impl Pipeline<Vec<FuncTokens>, FuncTokensTree> for FuncTokensToTree {
	fn apply(self, routes: Vec<FuncTokens>) -> FuncTokensTree {
		let mut this = FuncTokensTree::new("root");
		for route in routes {
			// 	// should be ancestors
			// 	// let parts = ;
			let mut current = &mut this;
			for component in route.route_info.path.components() {
				match component {
					std::path::Component::Normal(os_str)
						if let Some(str) = os_str.to_str() =>
					{
						current = VecExt::entry_or_insert_with(
							&mut current.children,
							|child| child.name == str,
							|| FuncTokensTree::new(str),
						);
					}
					_ => {} // std::path::Component::Prefix(prefix_component) => todo!(),
					        // std::path::Component::RootDir => todo!(),
					        // std::path::Component::CurDir => todo!(),
					        // std::path::Component::ParentDir => todo!(),
				}
			}
			current.value = Some(route);
		}
		this
	}
}



#[derive(Debug, Clone)]
pub struct FuncTokensTree {
	/// The route path for this part of the tree. It may be
	/// a parent or leaf node.
	pub name: String,
	pub value: Option<FuncTokens>,
	/// Children mapped by their [`RouteTreeBuilder::name`].
	/// If this is empty then the route is a leaf node.
	pub children: Vec<FuncTokensTree>,
}

impl FuncTokensTree {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			value: None,
			children: Vec::new(),
		}
	}

	/// usually for debugging, just output all paths
	/// and the route names
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
					#item
					#(#children)*
				}
			)
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	fn routes() -> Vec<FuncTokens> {
		vec![
			FuncTokens::simple("index.rs", syn::parse_quote!({})),
			FuncTokens::simple("foo/bar.rs", syn::parse_quote!({})),
			FuncTokens::simple("foo/bazz/index.rs", syn::parse_quote!({})),
			FuncTokens::simple("foo/bazz/boo.rs", syn::parse_quote!({})),
		]
	}

	#[test]
	fn correct_tree_structure() {
		expect(
			routes()
				.xpipe(FuncTokensToTree)
				.into_path_tree()
				.to_string_indented(),
		)
		.to_be(
			r#"root
  foo
    bar
    bazz
      boo
"#,
		);
	}
}
