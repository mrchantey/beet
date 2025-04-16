use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use syn::Expr;
use syn::Item;
use syn::ItemFn;



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
#[derive(Debug, Default, Clone)]
pub struct FuncTokensTreeToRouteTree {
	pub codegen_file: CodegenFile,
}

// TODO this should be RouteInfo not FuncTokens? we dont need the func body
impl Pipeline<FuncTokensTree, Result<()>> for FuncTokensTreeToRouteTree {
	fn apply(mut self, tree: FuncTokensTree) -> Result<()> {
		let paths_mod = self.routes_mod_tree(&tree);
		let collect_func = self.collect_func(&tree);
		self.codegen_file.add_item(paths_mod);
		self.codegen_file.add_item(collect_func);
		self.codegen_file.build_and_write()?;
		Ok(())
	}
}


impl FuncTokensTreeToRouteTree {
	fn into_path_func(tree: &FuncTokensTree) -> Option<ItemFn> {
		let Some(route) = &tree.value else {
			return None;
		};
		let route_ident = if tree.children.is_empty() {
			syn::Ident::new(
				&route.name().to_snake_case(),
				proc_macro2::Span::call_site(),
			)
		} else {
			syn::Ident::new("index", proc_macro2::Span::call_site())
		};
		let route_path = route.route_info.path.to_string_lossy().to_string();
		Some(syn::parse_quote!(
			/// Get the local route path
			pub fn #route_ident()->&'static str{
				#route_path
			}
		))
	}

	fn routes_mod_tree(&self, tree: &FuncTokensTree) -> Item {
		tree.mod_tree(|node| {
			Self::into_path_func(node)
				.map(|n| n.into())
				.unwrap_or(Item::Verbatim(TokenStream::default()))
		})
	}

	fn collect_func(&self, tree: &FuncTokensTree) -> ItemFn {
		let route_tree = self.collect_route_node(tree);
		syn::parse_quote!(
			/// Collect the static route tree
			pub fn collect() -> RoutePathTree {
				#route_tree
			}
		)
	}

	fn collect_route_node(&self, tree: &FuncTokensTree) -> Expr {
		let children = tree
			.children
			.iter()
			.map(|child| self.collect_route_node(child))
			.collect::<Vec<_>>();

		let path = match &tree.value {
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
	use quote::ToTokens;
	use sweet::prelude::*;
	use syn::ItemFn;
	use syn::ItemMod;

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
		let mod_item =
			FuncTokensTreeToRouteTree::default().routes_mod_tree(&tree);

		let expected: ItemMod = syn::parse_quote! {
		#[allow(missing_docs)]
		pub mod root {
			/// Get the local route path
			pub fn index() -> &'static str {
					"/"
			}
			#[allow(missing_docs)]
			pub mod foo {
					/// Get the local route path
					pub fn bar() -> &'static str {
							"/foo/bar"
					}
					#[allow(missing_docs)]
					pub mod bazz {
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
		let func = FuncTokensTreeToRouteTree::default().collect_func(&tree);

		let expected: ItemFn = syn::parse_quote! {
			/// Collect the static route tree
			pub fn collect() -> RoutePathTree {
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
