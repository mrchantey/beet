use crate::prelude::*;
use bevy::prelude::*;
use heck::ToSnakeCase;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use syn::Expr;
use syn::Ident;
use syn::Item;
use syn::ItemFn;
use syn::parse_quote;

/// Create a tree of routes from a list of [`FuncTokens`],
/// that can then be converted to an [`ItemMod`] to be used in the router.
///
/// ## Example
/// This is an example output for the following input files
///
/// - `index.rs`
/// - `foo/bar/index.rs`
/// - `foo/bar/bazz.rs`
///
/// ```
/// mod paths{
/// 	pub fn index()->&'static str{
/// 		"/"
/// 	}
/// 	// foo has no index file
/// 	mod foo{
/// 	 	mod bar{
///  			pub fn index()->&'static str{
/// 				"/foo/bar"
/// 			}
/// 			pub fn bazz()->&'static str{
/// 				"/foo/bar/bazz"
/// 			}
/// 		}
/// 	}
/// }
/// ```
pub fn parse_route_tree(
	mut query: Populated<(Entity, &mut CodegenFile), Changed<RouteCodegenRoot>>,
	collections: Query<(Entity, &RouteFileCollection)>,
	methods: Query<&RouteFileMethod>,
	children: Query<&Children>,
) {
	for (entity, mut codegen) in query.iter_mut() {
		let child_methods = children
			.iter_descendants(entity)
			.filter_map(|e| collections.get(e).ok())
			.filter(|(_, collection)| {
				collection.category.include_in_route_tree()
			})
			.map(|(e, _)| {
				children
					.iter_descendants(e)
					.filter_map(|c| methods.get(c).map(|m| (c, m)).ok())
			})
			.flatten()
			.collect::<Vec<_>>();

		let tree = RouteFileMethodTree::from_methods(child_methods);
		let parser = Parser { query: methods };
		codegen.add_item(parser.routes_mod_tree(&tree));
		codegen.add_item(parser.collect_func(&tree));
	}
}

#[derive(Clone)]
struct Parser<'w, 's, 'a> {
	query: Query<'w, 's, &'a RouteFileMethod>,
}

impl<'a> Parser<'_, '_, 'a> {
	fn routes_mod_tree(&self, tree: &RouteFileMethodTree) -> Item {
		tree.mod_tree(move |node| {
			self.clone()
				.tree_path_func(node)
				.map(|n| n.into())
				.unwrap_or(Item::Verbatim(TokenStream::default()))
		})
	}
	fn get(&self, entity: Entity) -> &RouteFileMethod {
		self.query.get(entity).expect(
			"Malformed RouteFileTree, entity does not have a RouteFileMethod component",
		)
	}

	fn collect_func(&self, tree: &RouteFileMethodTree) -> ItemFn {
		let route_tree = self.collect_route_node(tree);
		syn::parse_quote!(
			/// Collect the static route tree
			pub fn route_path_tree() -> RoutePathTree {
				#route_tree
			}
		)
	}

	fn tree_path_func(self, tree: &RouteFileMethodTree) -> Option<ItemFn> {
		// just use the first method, each func should have the same route path
		let Some(route) = &tree.funcs.iter().next() else {
			return None;
		};
		let route = self.get(**route);
		// if there are no children just use the name of the route,
		// otherwise specify `index` as the name
		let route_ident = if tree.children.is_empty() {
			let name = route
				.route_info
				.path
				.file_stem()
				.map(|s| s.to_string_lossy().to_snake_case())
				.unwrap_or("index".to_string());
			Ident::new(&name, Span::call_site())
		} else {
			Ident::new("index", Span::call_site())
		};
		let route_path = route.route_info.path.to_string_lossy().to_string();
		Some(parse_quote!(
			/// Get the local route path
			pub fn #route_ident()->&'static str{
				#route_path
			}
		))
	}

	fn collect_route_node(&self, tree: &RouteFileMethodTree) -> Expr {
		let children = tree
			.children
			.iter()
			.map(|child| self.collect_route_node(child))
			.collect::<Vec<_>>();

		let path = match &tree.funcs.iter().next() {
			Some(value) => {
				let value = self.get(**value);
				let path = value.route_info.path.to_string_lossy().to_string();
				let path: Expr = syn::parse_quote!(Some(RoutePath::new(#path)));
				path
			}
			None => {
				syn::parse_quote!(None)
			}
		};

		let name = &tree.name;

		syn::parse_quote!(RoutePathTree {
			name: #name.into(),
			path: #path,
			children: vec![#(#children),*],
		})
	}
}

#[cfg(test)]
mod test {
	use super::Parser;
	use crate::prelude::*;
	use beet_bevy::prelude::WorldMutExt;
	use beet_net::prelude::RouteInfo;
	use bevy::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;
	use syn::ItemFn;
	use syn::ItemMod;

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
	fn creates_mod() {
		let mut world = world();
		let methods = world
			.query_once::<(Entity, &RouteFileMethod)>()
			.iter()
			.copied()
			.collect();
		let tree = RouteFileMethodTree::from_methods(methods);
		let mut query = world.query::<&RouteFileMethod>();
		let query = query.query(&world);
		let mod_item = Parser { query }.routes_mod_tree(&tree);

		let expected: ItemMod = syn::parse_quote! {
			#[allow(missing_docs)]
			pub mod routes {
				#[allow(unused_imports)]
				use super::*;
				#[doc = r" Get the local route path"]
				pub fn index() -> &'static str {
					"/"
				}
				#[doc = r" Get the local route path"]
				pub fn bazz() -> &'static str {
					"/bazz"
				}
				#[allow(missing_docs)]
				pub mod foo {
					#[allow(unused_imports)]
					use super::*;
					#[doc = r" Get the local route path"]
					pub fn bar() -> &'static str {
						"/foo/bar"
					}
					#[allow(missing_docs)]
					pub mod bazz {
						#[allow(unused_imports)]
						use super::*;
						#[doc = r" Get the local route path"]
						pub fn index() -> &'static str {
							"/foo/bazz"
						}
						#[doc = r" Get the local route path"]
						pub fn boo() -> &'static str {
							"/foo/bazz/boo"
						}
					}
				}
			}
		};
		expect(mod_item.to_token_stream().to_string())
			.to_be_str(expected.to_token_stream().to_string());
	}
	#[test]
	fn creates_collect_tree() {
		let mut world = world();
		let methods = world
			.query_once::<(Entity, &RouteFileMethod)>()
			.iter()
			.copied()
			.collect();
		let tree = RouteFileMethodTree::from_methods(methods);
		let mut query = world.query::<&RouteFileMethod>();
		let query = query.query(&world);
		let func = Parser { query }.collect_func(&tree);

		let expected: ItemFn = syn::parse_quote! {
			#[doc = r" Collect the static route tree"]
			pub fn route_path_tree() -> RoutePathTree {
				RoutePathTree {
					name: "routes".into(),
					path: Some(RoutePath::new("/")),
					children: vec![
						RoutePathTree {
							name: "bazz".into(),
							path: Some(RoutePath::new("/bazz")),
							children: vec![],
						},
						RoutePathTree {
							name: "foo".into(),
							path: None,
							children: vec![
								RoutePathTree {
									name: "bar".into(),
									path: Some(RoutePath::new("/foo/bar")),
									children: vec![],
								},
								RoutePathTree {
									name: "bazz".into(),
									path: Some(RoutePath::new("/foo/bazz")),
									children: vec![
										RoutePathTree {
											name: "boo".into(),
											path: Some(RoutePath::new("/foo/bazz/boo")),
											children: vec![],
										}
									],
								}
							],
						}
					],
				}
			}
		};
		expect(func.to_token_stream().to_string())
			.to_be_str(expected.to_token_stream().to_string());
	}
}
