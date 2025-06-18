use crate::prelude::*;
use beet_common::prelude::TempNonSendMarker;
use bevy::prelude::*;
use heck::ToSnakeCase;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use syn::Expr;
use syn::Ident;
use syn::Item;
use syn::ItemFn;
use syn::parse_quote;

/// Create a tree of routes from a list of [`FuncTokens`]`,
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
#[derive(Debug, Default, Clone, Component)]
pub struct ParseRouteTree;


pub fn parse_route_tree(
	_: TempNonSendMarker,
	mut query: Populated<(Entity, &mut CodegenFileSendit, &ParseRouteTree)>,
	file_groups: Query<(Entity, &FileGroupSendit)>,
	methods: Query<&RouteFileMethod>,
	children: Query<&Children>,
) {
	for (entity, mut codegen, parse_tree) in query.iter_mut() {
		let methods = children
			.iter_descendants(entity)
			.filter_map(|e| file_groups.get(e).ok())
			.filter(|(_, group)| group.route_tree)
			.map(|(e, _)| {
				children
					.iter_descendants(e)
					.filter_map(|c| methods.get(c).ok().map(|m| m.clone()))
			})
			.flatten()
			.collect::<Vec<_>>();

		let tree = RouteFileMethodTree::from_methods(methods);
		codegen.add_item(parse_tree.routes_mod_tree(&tree));
		codegen.add_item(parse_tree.collect_func(&tree));
	}
}

impl ParseRouteTree {
	fn routes_mod_tree(&self, tree: &RouteFileMethodTree) -> Item {
		tree.mod_tree(|node| {
			Self::tree_path_func(node)
				.map(|n| n.into())
				.unwrap_or(Item::Verbatim(TokenStream::default()))
		})
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

	fn tree_path_func(tree: &RouteFileMethodTree) -> Option<ItemFn> {
		// just use the first method, each func should have the same route path
		let Some(route) = &tree.funcs.iter().next() else {
			return None;
		};
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
	use crate::prelude::*;
	use beet_net::prelude::RouteInfo;
	use quote::ToTokens;
	use sweet::prelude::*;
	use syn::ItemFn;
	use syn::ItemMod;

	fn tree() -> RouteFileMethodTree {
		vec![
			RouteFileMethod::new("/"),
			RouteFileMethod::new("/foo/bar"),
			RouteFileMethod::new("/foo/bazz"),
			RouteFileMethod::new("/foo/bazz/boo"),
			RouteFileMethod::new(RouteInfo::post("foo/bazz/boo")),
		]
		.into()
	}

	#[test]
	fn correct_tree_structure() {
		expect(tree().into_path_tree().to_string_indented()).to_be(
			r#"root
  foo
    bar
    bazz
      boo
"#,
		);
	}
	#[test]
	fn creates_mod() {
		let tree = tree();
		let mod_item = ParseRouteTree::default().routes_mod_tree(&tree);

		let expected: ItemMod = syn::parse_quote! {
		#[allow(missing_docs)]
		pub mod root {
			#[allow (unused_imports)]
			use super::*;
			/// Get the local route path
			pub fn index() -> &'static str {
				"/"
			}
			#[allow(missing_docs)]
			pub mod foo {
					#[allow (unused_imports)]
					use super::*;
					/// Get the local route path
					pub fn bar() -> &'static str {
						"/foo/bar"
					}
					#[allow(missing_docs)]
					pub mod bazz {
							#[allow (unused_imports)]
							use super::*;
							/// Get the local route path
							pub fn index() -> &'static str {
									"/foo/bazz"
							}

							/// Get the local route path
							pub fn boo() -> &'static str {
									"/foo/bazz/boo"
							}
					}
			}
		}
				};
		expect(mod_item.to_token_stream().to_string())
			.to_be(expected.to_token_stream().to_string());
	}
	#[test]
	fn creates_collect_tree() {
		let tree = tree();
		let func = ParseRouteTree::default().collect_func(&tree);

		let expected: ItemFn = syn::parse_quote! {
			/// Collect the static route tree
			pub fn route_path_tree() -> RoutePathTree {
				RoutePathTree {
						name: "root".into(),
						path: Some(RoutePath::new("/")),
						children: vec![
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
			.to_be(expected.to_token_stream().to_string());
	}
}
