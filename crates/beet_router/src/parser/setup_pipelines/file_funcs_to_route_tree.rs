use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::RsxPipeline;
use rapidhash::RapidHashMap;
use serde::Deserialize;
use serde::Serialize;
use syn::Expr;
use syn::Item;
use syn::ItemFn;
use syn::ItemMod;


/// Create a tree of routes from a list of [`FileFuncs`]`,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFuncsToRouteTree {
	pub codgen_file: CodegenFile,
}

impl RsxPipeline<Vec<FileFuncs>, Result<()>> for FileFuncsToRouteTree {
	fn apply(self, value: Vec<FileFuncs>) -> Result<()> {
		let tree = RouteTreeBuilder::from_files(value.iter());
		let mut codegen_file = self.codgen_file;
		codegen_file.add_item(tree.into_paths_mod());
		codegen_file.add_item(tree.into_collect_static_route_tree());
		codegen_file.build_and_write()?;
		Ok(())
	}
}



#[derive(Debug, Clone, PartialEq, Eq)]
struct RouteTreeBuilder<'a> {
	name: String,
	files: Vec<&'a FileFuncs>,
	children: RapidHashMap<String, RouteTreeBuilder<'a>>,
}

impl<'a> RouteTreeBuilder<'a> {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			files: Default::default(),
			children: Default::default(),
		}
	}

	pub fn from_files(files: impl Iterator<Item = &'a FileFuncs>) -> Self {
		let mut tree = Self::new("root");
		for file in files {
			let parts = file.route_path.to_string_lossy().to_string();
			let parts = parts
				.split('/')
				.filter(|p| !p.is_empty())
				.collect::<Vec<_>>();
			let num_to_remove = if file.local_path.ends_with("index.rs") {
				0
			} else {
				1
			};


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
			current.files.push(file);
		}
		tree
	}

	pub fn into_paths_mod(&self) -> ItemMod {
		self.into_paths_mod_inner("paths")
	}
	fn into_paths_mod_inner(&self, name: &str) -> ItemMod {
		let mod_items = self
			.files
			.iter()
			.map(|file| {
				let ident =
					syn::Ident::new(&file.name, proc_macro2::Span::call_site());
				let route_path = file.route_path.to_string_lossy().to_string();
				let item: Item = syn::parse_quote!(
					/// Get the local route path
					pub fn #ident()->&'static str{
						#route_path
					}
				);
				item
			})
			.chain(
				self.children.iter().map(|(name, child)| {
					child.into_paths_mod_inner(name).into()
				}),
			);

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

		let paths = self.files.iter().map(|file| {
			let path = file.route_path.to_string_lossy().to_string();
			let path: Expr = syn::parse_quote!(RoutePath::new(#path));
			path
		});
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
	use quote::ToTokens;
	use rapidhash::RapidHashMap;
	use sweet::prelude::*;
	use syn::ItemFn;
	use syn::ItemMod;

	use super::RouteTreeBuilder;


	fn files() -> Vec<FileFuncs> {
		vec![
			FileFuncs {
				name: "index".into(),
				local_path: "index.rs".into(),
				route_path: "/".into(),
				canonical_path: Default::default(),
				funcs: Default::default(),
			},
			FileFuncs {
				name: "index".into(),
				local_path: "foo/bar/index.rs".into(),
				route_path: "/foo/bar".into(),
				canonical_path: Default::default(),
				funcs: Default::default(),
			},
			// respects route path over local path
			FileFuncs {
				name: "bazz".into(),
				local_path: "bazz.booboo.rs".into(),
				route_path: "/foo/bar/bazz".into(),
				canonical_path: Default::default(),
				funcs: Default::default(),
			},
		]
	}

	#[test]
	fn creates_nodes() {
		let files = files();
		let tree = RouteTreeBuilder::from_files(files.iter());

		// #[rustfmt::skip]
		expect(tree).to_be(RouteTreeBuilder {
			name: "root".to_string(),
			children: RapidHashMap::from_iter(vec![(
				"foo".to_string(),
				RouteTreeBuilder {
					name: "foo".to_string(),
					children: RapidHashMap::from_iter(vec![(
						"bar".to_string(),
						RouteTreeBuilder {
							name: "bar".to_string(),
							children: RapidHashMap::from_iter(vec![]),
							files: vec![&files[1], &files[2]],
						},
					)]),
					files: vec![],
				},
			)]),
			files: vec![&files[0]],
		});
	}

	#[test]
	fn creates_mod() {
		let files = files();
		let tree = RouteTreeBuilder::from_files(files.iter());
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
		let files = files();
		let tree = RouteTreeBuilder::from_files(files.iter());
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
										RoutePath::new("/foo/bar"),
										RoutePath::new("/foo/bar/bazz")
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
