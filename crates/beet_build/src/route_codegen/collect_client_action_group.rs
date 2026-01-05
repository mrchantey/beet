use crate::prelude::*;
use beet_core::prelude::*;
use proc_macro2::Span;
use syn::Item;

/// Added as a child of any [`RouteFileCollection`] with a [`RouteFileCategory::Action`],
/// meaning a client actions codegen will be created.
#[derive(Debug, Clone, Reflect, Component)]
#[reflect(Component)]
#[require(CodegenFile)]
pub struct CollectClientActions {
	/// Collapse single child functions into their parent mod
	pub collapse_nodes: bool,
}

impl Default for CollectClientActions {
	fn default() -> Self {
		Self {
			collapse_nodes: true,
		}
	}
}

pub fn collect_client_action_group(
	mut query: Populated<
		(&mut CodegenFile, &CollectClientActions, &ChildOf),
		Added<CodegenFile>,
	>,
	children: Query<&Children>,
	methods: Query<&RouteFileMethod>,
) {
	for (mut codegen_file, collect, childof) in query.iter_mut() {
		let child_methods = children
			.iter_descendants(childof.parent())
			.filter_map(|child| {
				methods.get(child).map(|method| (child, method)).ok()
			})
			.collect::<Vec<_>>();
		debug!("Collecting {} client actions", child_methods.len());
		let tree = RouteFileMethodTree::from_methods(child_methods);

		let item = Builder {
			collect,
			query: methods,
		}
		.mod_tree(&tree);
		codegen_file.add_item(item);
		drop(codegen_file);
	}
}


struct Builder<'w, 's, 'a, 'b> {
	collect: &'a CollectClientActions,
	query: Query<'w, 's, &'b RouteFileMethod>,
}

impl Builder<'_, '_, '_, '_> {
	fn get(&self, entity: Entity) -> &RouteFileMethod {
		self.query.get(entity).expect(
			"Malformed RouteFileTree, entity does not have a RouteFileMethod component",
		)
	}

	/// Create a tree of server actions
	fn mod_tree(&self, tree: &RouteFileMethodTree) -> Item {
		let item = self.mod_tree_inner(tree);
		if self.collect.collapse_nodes {
			self.collapse_item(item)
		} else {
			item
		}
	}
	fn mod_tree_inner(&self, tree: &RouteFileMethodTree) -> Item {
		let ident = syn::Ident::new(&tree.name.to_string(), Span::call_site());
		let children =
			tree.children.iter().map(|child| self.mod_tree_inner(child));

		let items = tree.funcs.iter().map(|tokens| {
			let method = self.get(*tokens);
			ParseClientAction.client_func(&method)
		});

		syn::parse_quote! {
			#[allow(missing_docs)]
			pub mod #ident {
				#[allow(unused_imports)]
				use super::*;
				#(#items)*
				#(#children)*
			}
		}
	}



	/// Recursive collapse, if this item is a mod and its only child is a function,
	/// we can collapse it so that the function replaces the mod
	/// and its name becomes the mod name
	///
	/// For example:
	/// ```ignore
	/// mod foo {
	/// 	 use super::*;
	/// 	 fn foo() {}
	/// }
	/// ```
	/// becomes:
	/// ```ignore
	/// fn foo() {}
	/// ```
	fn collapse_item(&self, item: Item) -> Item {
		if let Item::Mod(mut item_mod) = item {
			if let Some((_, ref mut items)) = item_mod.content {
				// the first is `use super::*;`
				if items.len() == 2 {
					if let Item::Fn(mut func) = items[1].clone() {
						func.sig.ident = item_mod.ident;
						return Item::Fn(func);
					}
				}
				// otherwise map all children
				*items = items
					.drain(..)
					.map(|item| self.collapse_item(item.clone()))
					.collect();
			}
			Item::Mod(item_mod)
		} else {
			item
		}
	}
}


#[cfg(test)]
mod test {
	use super::Builder;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use proc_macro2::TokenStream;
	use quote::ToTokens;

	fn mod_tree(methods: Vec<RouteFileMethod>) -> TokenStream {
		let mut world = World::new();
		world.spawn_batch(methods);

		let methods = world
			.query_once::<(Entity, &RouteFileMethod)>()
			.iter()
			.copied()
			.collect();
		let tree = RouteFileMethodTree::from_methods(methods);

		let mut query = world.query::<&RouteFileMethod>();
		let query = query.query(&world);
		let builder = Builder {
			collect: &CollectClientActions::default(),
			query,
		};
		builder.mod_tree(&tree).xmap(|item| item.to_token_stream())
	}

	#[test]
	fn simple() {
		mod_tree(vec![RouteFileMethod::new("/bazz")]).xpect_snapshot();
	}


	#[test]
	fn correct_tree_structure() {
		mod_tree(vec![
			RouteFileMethod::new("bazz"),
			RouteFileMethod::new("foo/bar"),
			RouteFileMethod::new("foo/boo"),
			RouteFileMethod::new(RouteInfo::post("foo/boo")),
			RouteFileMethod::new(RouteInfo::post("foo/bing/bong")),
		])
		.xpect_snapshot();
	}
}
