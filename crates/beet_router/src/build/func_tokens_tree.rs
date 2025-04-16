use crate::prelude::*;
use sweet::prelude::*;
use syn::Item;

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

	/// Flattens the tree into a [`FuncTokensGroup`].
	pub fn into_group(self) -> FuncTokensGroup { self.into() }
}

impl Into<FuncTokensGroup> for FuncTokensTree {
	fn into(self) -> FuncTokensGroup {
		let mut out = Vec::new();
		if let Some(value) = self.value {
			out.push(value);
		}
		for child in self.children.into_iter() {
			out.extend(child.into_group().funcs);
		}
		FuncTokensGroup::new(out)
	}
}

impl From<Vec<FuncTokens>> for FuncTokensTree {
	fn from(value: Vec<FuncTokens>) -> Self {
		FuncTokensGroup::new(value).into()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	fn tree() -> FuncTokensTree {
		vec![
			FuncTokens::simple("index.rs"),
			FuncTokens::simple("foo/bar.rs"),
			FuncTokens::simple("foo/bazz/index.rs"),
			FuncTokens::simple("foo/bazz/boo.rs"),
		]
		.into()
	}

	#[test]
	fn correct_tree_structure() {
		expect(
			tree()
				.xinto::<FuncTokensTree>()
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
