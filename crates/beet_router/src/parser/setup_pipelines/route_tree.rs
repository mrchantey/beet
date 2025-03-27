use crate::prelude::*;
use rapidhash::RapidHashMap;
use syn::Item;
use syn::ItemMod;


/// Create a tree of routes from a list of [`FileFuncs`]`,
/// that can then be converted to an [`ItemMod`] to be used in the router.
///
/// ## Example
/// This is an example output for the following input files
/// ```
///
/// let files = vec![
/// 	FileFuncs::new("index.rs", "/"),
/// 	FileFuncs::new("foo/bar/index.rs", "/foo/bar"),
/// 	FileFuncs::new("foo/bar/bazz.rs", "/foo/bar/bazz"),
/// ];
///
/// mod paths{
/// 	pub fn index()->&'static str{
/// 		"/"
/// 	}
/// 	// foo has no index file
/// 	mod foo{
/// 	 	mod bar{
///  			pub fn index()->&'static str{
/// 				"/foo/bar/index.rs"
/// 			}
/// 			pub fn bazz()->&'static str{
/// 				"/foo/bar/bazz"
/// 			}
/// 		}
/// 	}
/// }
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RouteTree<'a> {
	children: RapidHashMap<String, RouteTree<'a>>,
	files: Vec<&'a FileFuncs>,
}

impl<'a> RouteTree<'a> {
	pub fn new(files: impl Iterator<Item = &'a FileFuncs>) -> Self {
		let mut tree = Self::default();
		for file in files {
			let parts = file.local_path.to_string_lossy().to_string();
			let parts = parts
				.split('/')
				.filter(|p| !p.is_empty())
				.collect::<Vec<_>>();

			let mut current = &mut tree;
			// For each part of the path except the last one, create nodes
			for part in parts.iter().take(parts.len().saturating_sub(1)) {
				current = current.children.entry(part.to_string()).or_default();
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
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::ToTokens;
	use rapidhash::RapidHashMap;
	use sweet::prelude::*;
	use syn::ItemMod;


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
			FileFuncs {
				name: "bazz".into(),
				local_path: "foo/bar/bazz.booboo.rs".into(),
				route_path: "/foo/bar/bazz".into(),
				canonical_path: Default::default(),
				funcs: Default::default(),
			},
		]
	}

	#[test]
	fn creates_nodes() {
		let files = files();
		let tree = RouteTree::new(files.iter());

		// #[rustfmt::skip]
		expect(tree).to_be(RouteTree {
			children: RapidHashMap::from_iter(vec![(
				"foo".to_string(),
				RouteTree {
					children: RapidHashMap::from_iter(vec![(
						"bar".to_string(),
						RouteTree {
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
		let tree = RouteTree::new(files.iter());
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
}
