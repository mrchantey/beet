use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::RsxPipeline;
use rapidhash::RapidHashMap;
use rapidhash::RapidHashSet;
use syn::Expr;
use syn::Item;
use syn::ItemFn;
use syn::ItemMod;


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

impl RsxPipeline<Vec<FuncTokens>, Result<()>> for RouteFuncsToTree {
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
	name: String,
	routes: Vec<&'a FuncTokens>,
	children: RapidHashMap<String, RouteTreeBuilder<'a>>,
}

impl<'a> RouteTreeBuilder<'a> {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			routes: Default::default(),
			children: Default::default(),
		}
	}

	pub fn from_routes(routes: impl Iterator<Item = &'a FuncTokens>) -> Self {
		let mut tree = Self::new("root");
		for route in routes {
			let parts = route.route_info.path.to_string_lossy().to_string();
			let parts = parts
				.split('/')
				.filter(|p| !p.is_empty())
				.collect::<Vec<_>>();
			let num_to_remove = if route.is_index() { 0 } else { 1 };

			let mut current = &mut tree;
			// For each part of the path except the last one, create nodes
			for part in
				parts.iter().take(parts.len().saturating_sub(num_to_remove))
			{
				current = current
					.children
					.entry(part.to_string())
					.or_insert_with(|| RouteTreeBuilder::new(*part));
			}
			// Add the file to the final node
			current.routes.push(route);
		}
		tree
	}

	pub fn into_paths_mod(&self) -> ItemMod {
		self.into_paths_mod_inner("paths")
	}
	fn into_paths_mod_inner(&self, name: &str) -> ItemMod {
		let mod_items =
			self.routes
				.iter()
				.map(|route| {
					let ident = syn::Ident::new(
						&route.name(),
						proc_macro2::Span::call_site(),
					);
					let route_path =
						route.route_info.path.to_string_lossy().to_string();
					let item: Item = syn::parse_quote!(
						/// Get the local route path
						pub fn #ident()->&'static str{
							#route_path
						}
					);
					item
				})
				.chain(self.children.iter().map(|(name, child)| {
					child.into_paths_mod_inner(name).into()
				}));

		let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
		syn::parse_quote!(
			/// Nested local route paths
			pub mod #ident {
				#(#mod_items)*
			}
		)
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
			.values()
			.map(|child| child.into_static_route_tree())
			.collect::<Vec<_>>();

		let paths = self
			.routes
			.iter()
			.map(|func| {
				let path = func.route_info.path.to_string_lossy().to_string();
				let path: Expr = syn::parse_quote!(RoutePath::new(#path));
				path
			})
			.collect::<Vec<_>>()
			.into_iter()
			.collect::<RapidHashSet<_>>()
			.into_iter();

		let name = &self.name;

		syn::parse_quote!(StaticRouteTree {
			name: #name.into(),
			paths: vec![#(#paths),*],
			children: vec![#(#children),*],
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use http::Method;
	use quote::ToTokens;
	use quote::quote;
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
				path: RoutePath::parse_local_path(local_path).unwrap(),
				method: Method::GET,
			},
			local_path: local_path.into(),
		}
	}

	fn routes() -> Vec<FuncTokens> {
		vec![
			route("index.rs"),
			route("foo/bar/index.rs"),
			route("foo/bar/bazz.rs"),
		]
	}

	#[test]
	fn creates_nodes() {
		let files = routes();
		let tree = RouteTreeBuilder::from_routes(files.iter());

		// #[rustfmt::skip]
		expect(tree.into_static_route_tree().to_token_stream().to_string())
			.to_be(
				quote! {
					StaticRouteTree {
						name: "root".into(),
						paths: vec![RoutePath::new("/")],
						children: vec![
							StaticRouteTree {
								name: "foo".into(),
								paths: vec![],
								children: vec![
									StaticRouteTree {
										name: "bar".into(),
										paths: vec![
											RoutePath::new("/foo/bar/bazz"),
											RoutePath::new("/foo/bar")
										],
										children: vec![],
									}
								],
							}
						],
					}
				}
				.to_string(),
			);
	}

	#[test]
	fn creates_mod() {
		let routes = routes();
		let tree = RouteTreeBuilder::from_routes(routes.iter());
		let mod_item = tree.into_paths_mod();

		let expected: ItemMod = syn::parse_quote! {
			/// Nested local route paths
			pub mod paths {
				/// Get the local route path
				pub fn index()->&'static str{
					"/"
				}
				/// Nested local route paths
				pub mod foo {
					/// Nested local route paths
					pub mod bar {
						/// Get the local route path
						pub fn index()->&'static str{
							"/foo/bar"
						}
						/// Get the local route path
						pub fn bazz()->&'static str{
							"/foo/bar/bazz"
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
					paths: vec![
						RoutePath::new("/")
						],
					children: vec![
							StaticRouteTree {
							name: "foo".into(),
							paths: vec![],
							children: vec![
								StaticRouteTree {
									name: "bar".into(),
									paths: vec![
										RoutePath::new("/foo/bar/bazz"),
										RoutePath::new("/foo/bar")
									],
									children: vec![],
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
