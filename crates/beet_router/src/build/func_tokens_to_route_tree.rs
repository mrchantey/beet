use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use heck::ToSnakeCase;
use quote::ToTokens;
use sweet::prelude::*;
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
#[derive(Debug, Clone)]
pub struct RouteFuncsToTree {
	pub codgen_file: CodegenFile,
}

impl Pipeline<Vec<FuncTokens>, Result<()>> for RouteFuncsToTree {
	fn apply(self, value: Vec<FuncTokens>) -> Result<()> {
		let tree = RouteTreeBuilder::from_routes(value.iter());
		let mut codegen_file = self.codgen_file;
		codegen_file.add_item(tree.into_paths_mod());
		codegen_file.add_item(tree.into_collect_static_route_tree());
		codegen_file.build_and_write()?;
		Ok(())
	}
}



#[derive(Debug, Clone)]
struct RouteTreeBuilder<'a> {
	/// The route path for this part of the tree. It may be
	/// a parent or leaf node.
	name: String,
	value: Option<&'a FuncTokens>,
	/// Children mapped by their [`RouteTreeBuilder::name`].
	/// If this is empty then the route is a leaf node.
	children: Vec<RouteTreeBuilder<'a>>,
}

impl<'a> RouteTreeBuilder<'a> {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			value: None,
			children: Vec::new(),
		}
	}

	pub fn from_routes(routes: impl Iterator<Item = &'a FuncTokens>) -> Self {
		let mut this = Self::new("root");
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
							|| RouteTreeBuilder::new(str),
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

	/// usually for debugging, just output all paths
	/// and the route names
	#[allow(dead_code)]
	fn into_path_tree(&self) -> Tree<String> {
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

	fn into_path_func(&self) -> Option<ItemFn> {
		let Some(route) = &self.value else {
			return None;
		};
		let route_ident = if self.children.is_empty() {
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

	pub fn into_paths_mod(&self) -> Item {
		if self.children.is_empty() {
			self.into_path_func()
				.expect(
					"RouteTreeBuilders with no path and no children is not allowed",
				)
				.into()
		} else {
			let children =
				self.children.iter().map(|child| child.into_paths_mod());
			let ident =
				syn::Ident::new(&self.name, proc_macro2::Span::call_site());
			let path = self
				.into_path_func()
				.map(|p| p.to_token_stream())
				.unwrap_or_default();
			syn::parse_quote!(
				/// Nested local route paths
				pub mod #ident {
					#path
					#(#children)*
				}
			)
		}
	}

	pub fn into_collect_static_route_tree(&self) -> ItemFn {
		let route_tree = self.into_static_route_tree();
		syn::parse_quote!(
			/// Collect the static route tree
			pub fn collect_static_route_tree() -> StaticRouteTree {
				#route_tree
			}
		)
	}

	fn into_static_route_tree(&self) -> Expr {
		let children = self
			.children
			.iter()
			.map(|child| child.into_static_route_tree())
			.collect::<Vec<_>>();

		let path = match &self.value {
			Some(value) => {
				let path = value.route_info.path.to_string_lossy().to_string();
				let path: Expr = syn::parse_quote!(Some(RoutePath::new(#path)));
				path
			}
			None => {
				syn::parse_quote!(None)
			}
		};

		let name = &self.name;

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
	use http::Method;
	use quote::ToTokens;
	use sweet::prelude::*;
	use syn::ItemFn;
	use syn::ItemMod;

	use super::RouteTreeBuilder;

	fn route(local_path: &str) -> FuncTokens {
		FuncTokens {
			mod_ident: None,
			frontmatter: syn::parse_quote!({}),
			func: syn::parse_quote!({}),
			canonical_path: CanonicalPathBuf::new_unchecked(local_path),
			route_info: RouteInfo {
				path: RoutePath::from_file_path(local_path).unwrap(),
				method: Method::GET,
			},
			local_path: local_path.into(),
		}
	}

	fn routes() -> Vec<FuncTokens> {
		vec![
			route("index.rs"),
			route("foo/bar.rs"),
			route("foo/bazz/index.rs"),
			route("foo/bazz/boo.rs"),
		]
	}

	#[test]
	fn correct_tree_structure() {
		expect(
			RouteTreeBuilder::from_routes(routes().iter())
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
		let tree = RouteTreeBuilder::from_routes(routes.iter());
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
		let files = routes();
		let tree = RouteTreeBuilder::from_routes(files.iter());
		let func = tree.into_collect_static_route_tree();

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
