use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use syn::Expr;
use syn::ItemFn;


// TODO this should be RouteInfo not FuncTokens, we dont need the func body

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

impl Pipeline<FuncTokensTree, Result<()>> for FuncTokensTreeToRouteTree {
	fn apply(mut self, tree: FuncTokensTree) -> Result<()> {
		let paths_mod = tree.into_paths_mod();
		let collect_func = self.into_collect_route_tree(&tree);
		self.codegen_file.add_item(paths_mod);
		self.codegen_file.add_item(collect_func);
		self.codegen_file.build_and_write()?;
		Ok(())
	}
}


impl FuncTokensTreeToRouteTree {
	fn into_collect_route_tree(&self, tree: &FuncTokensTree) -> ItemFn {
		let route_tree = self.tokens_to_tree(tree);
		syn::parse_quote!(
			/// Collect the static route tree
			pub fn collect_static_route_tree() -> StaticRouteTree {
				#route_tree
			}
		)
	}

	fn tokens_to_tree(&self, tree: &FuncTokensTree) -> Expr {
		let children = tree
			.children
			.iter()
			.map(|child| self.tokens_to_tree(child))
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

		syn::parse_quote!(StaticRouteTree {
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
	#[test]
	fn creates_mod() {
		let routes = routes();
		let tree = routes.xpipe(FuncTokensToTree);
		let mod_item = tree.into_paths_mod();

		let expected: ItemMod = syn::parse_quote! {
		/// Nested local route paths
		pub mod root {
			/// Get the local route path
			pub fn index() -> &'static str {
					"/"
			}
			/// Nested local route paths
			pub mod foo {
					/// Get the local route path
					pub fn bar() -> &'static str {
							"/foo/bar"
					}
					/// Nested local route paths
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
		let tree = routes().xpipe(FuncTokensToTree);
		let func =
			FuncTokensTreeToRouteTree::default().into_collect_route_tree(&tree);

		let expected: ItemFn = syn::parse_quote! {
			/// Collect the static route tree
			pub fn collect_static_route_tree() -> StaticRouteTree {
				StaticRouteTree {
						name: "root".into(),
						path: Some(RoutePath::new("/")),
						children: vec![
								StaticRouteTree {
										name: "foo".into(),
										path: None,
										children: vec![
												StaticRouteTree {
														name: "bar".into(),
														path: Some(RoutePath::new("/foo/bar")),
														children: vec![],
												},
												StaticRouteTree {
														name: "bazz".into(),
														path: Some(RoutePath::new("/foo/bazz")),
														children: vec![
																StaticRouteTree {
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
