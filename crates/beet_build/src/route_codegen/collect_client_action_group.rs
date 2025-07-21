use crate::prelude::*;
use beet_utils::prelude::AbsPathBuf;
use bevy::prelude::*;
use syn::Item;
use syn::ItemMod;
use syn::ItemUse;

/// Added as a child of any [`RouteFileCollection`] with a [`RouteFileCategory::Action`],
/// meaning a client actions codegen will be created.
#[derive(Debug, Clone, Component)]
#[require(CodegenFile)]
pub struct CollectClientActions {
	/// Collapse single child functions into their parent mod
	pub collapse_nodes: bool,
}

impl CollectClientActions {
	pub fn path(actions_codegen: &AbsPathBuf) -> AbsPathBuf {
		let mut path = actions_codegen.clone();
		let stem = path
			.file_stem()
			.expect("Actions codegen path must have a file stem");
		{
			let stem = format!("client_{}.rs", stem.to_string_lossy());
			path.set_file_name(stem);
		}
		path
	}

	pub fn ident(actions_codegen: &AbsPathBuf) -> syn::Ident {
		let stem = actions_codegen
			.file_stem()
			.expect("Actions codegen path must have a file stem");
		quote::format_ident!("client_{}", stem.to_string_lossy())
	}
}

impl Default for CollectClientActions {
	fn default() -> Self {
		Self {
			collapse_nodes: true,
		}
	}
}


pub fn add_client_codegen_to_actions_export(
	query: Populated<&ChildOf, Changed<CollectClientActions>>,
	mut collection_codegen: Query<&mut CodegenFile, With<RouteFileCollection>>,
) -> Result {
	for child in query.iter() {
		let mut codegen = collection_codegen.get_mut(child.parent())?;

		let ident = CollectClientActions::ident(&codegen.output);
		codegen.add_item::<ItemMod>(syn::parse_quote! {
			pub mod #ident;
		});
		codegen.add_item::<ItemUse>(syn::parse_quote! {
			pub use #ident::routes::actions::*;
		});
	}
	Ok(())
}

pub fn collect_client_action_group(
	mut query: Populated<
		(&mut CodegenFile, &CollectClientActions, &ChildOf),
		Changed<CollectClientActions>,
	>,
	children: Query<&Children>,
	methods: Query<(&RouteFileMethod, &RouteFileMethodSyn)>,
) {
	for (mut codegen_file, collect, childof) in query.iter_mut() {
		let child_methods = children
			.iter_descendants(childof.parent())
			.filter_map(|child| {
				methods.get(child).map(|(r, _)| (child, r)).ok()
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


struct Builder<'w, 's, 'a, 'b, 'c> {
	collect: &'a CollectClientActions,
	query: Query<'w, 's, (&'b RouteFileMethod, &'c RouteFileMethodSyn)>,
}

impl Builder<'_, '_, '_, '_, '_> {
	fn get(&self, entity: Entity) -> (&RouteFileMethod, &RouteFileMethodSyn) {
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
		let ident = syn::Ident::new(&tree.name, proc_macro2::Span::call_site());
		let children =
			tree.children.iter().map(|child| self.mod_tree_inner(child));

		let items = tree.funcs.iter().map(|tokens| {
			let (route, func) = self.get(*tokens);
			ParseClientAction.client_func(&route.route_info, &func)
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
	use beet_core::prelude::WorldMutExt;
	use beet_utils::utils::PipelineTarget;
	use bevy::prelude::*;
	use quote::ToTokens;
	use quote::quote;
	use sweet::prelude::*;

	fn mod_tree(methods: Vec<(RouteFileMethod, RouteFileMethodSyn)>) -> String {
		let mut world = World::new();
		world.spawn_batch(methods);

		let methods = world
			.query_once::<(Entity, &RouteFileMethod)>()
			.iter()
			.copied()
			.collect();
		let tree = RouteFileMethodTree::from_methods(methods);

		let mut query =
			world.query::<(&RouteFileMethod, &RouteFileMethodSyn)>();
		let query = query.query(&world);
		let builder = Builder {
			collect: &CollectClientActions::default(),
			query,
		};
		builder
			.mod_tree(&tree)
			.xmap(|item| item.to_token_stream().to_string())
	}

	#[test]
	fn simple() {
		mod_tree(vec![(
			RouteFileMethod::new("/bazz"),
			RouteFileMethodSyn::new(syn::parse_quote!(
				fn get() {}
			)),
		)])
		.xpect()
		.to_be_str(
			quote! {
				#[allow(missing_docs)]
				pub mod routes {
					#[allow(unused_imports)]
					use super::*;
					pub async fn bazz() -> ServerActionResult<(), ()> {
						CallServerAction::request_no_data(RouteInfo {
							path: RoutePath(std::path::PathBuf::from("/bazz")),
							method: HttpMethod::Get
						}).await
					}
				}
			}
			.to_string(),
		);
	}


	#[test]
	fn correct_tree_structure() {
		mod_tree(vec![
			(
				RouteFileMethod::new("bazz"),
				RouteFileMethodSyn::new(syn::parse_quote!(
					fn get() {}
				)),
			),
			(
				RouteFileMethod::new("foo/bar"),
				RouteFileMethodSyn::new(syn::parse_quote!(
					fn get() {}
				)),
			),
			(
				RouteFileMethod::new("foo/boo"),
				RouteFileMethodSyn::new(syn::parse_quote!(
					fn get() {}
				)),
			),
			(
				RouteFileMethod::new("foo/boo"),
				RouteFileMethodSyn::new(syn::parse_quote!(
					fn post() {}
				)),
			),
			(
				RouteFileMethod::new("foo/bing/bong"),
				RouteFileMethodSyn::new(syn::parse_quote!(
					fn post() {}
				)),
			),
		])
		.xpect()
		.to_be_snapshot();
	}
}
