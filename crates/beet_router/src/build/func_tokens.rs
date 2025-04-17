#[allow(unused_imports)]
use crate::prelude::*;
use anyhow::Result;
use std::path::PathBuf;
use std::str::FromStr;
use sweet::prelude::*;
use syn::Block;
use syn::Ident;
use syn::ItemFn;
use syn::ItemMod;

/// Tokens for a function that may be used as a route. This may
/// be considered the `Tokens` version of a [`RouteFunc`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncTokens {
	/// A unique identifier for this file based on its index in
	/// the [`FileGroup`], ie `file1`. It is used for importing the file
	/// as a module by its path. We need this awkwardness because rust analyzer
	/// struggles to detect path imports nested inside a block.
	pub mod_ident: Ident,
	/// The strategy for the mod import, whether its linking to a file
	/// or defining the function inline.
	pub mod_import: ModImport,
	/// A block that returns the frontmatter of this function, this may be a unit type
	/// or [`None`] if the eventual type allows for it.
	pub frontmatter: Block,
	/// The function defined in the [`mod_ident`](Self::mod_ident) module.
	/// Its return type is the [`FuncTokensGroup::func_type`].
	pub item_fn: ItemFn,
	/// Canonical path to the file
	pub canonical_path: CanonicalPathBuf,
	/// Path relative to the [`src`](FileGroup::src) of the [`FileGroup`]
	pub local_path: PathBuf,
	/// A reasonable route path generated from this file's local path,
	/// and a method matching either the functions signature, or
	/// `get` in the case of single file routes like markdown.
	pub route_info: RouteInfo,
}
impl AsRef<FuncTokens> for FuncTokens {
	fn as_ref(&self) -> &FuncTokens { self }
}


impl FuncTokens {
	pub fn simple_get(local_path: impl AsRef<std::path::Path>) -> Self {
		Self::simple_with_func(local_path, syn::parse_quote! {
			fn get()->RsxNode{
				Default::default()
			}
		})
	}
	pub fn simple_post(local_path: impl AsRef<std::path::Path>) -> Self {
		Self::simple_with_func(local_path, syn::parse_quote! {
			fn post()->RsxNode{
				Default::default()
			}
		})
	}

	/// create a simple `FuncTokens` for testing
	pub fn simple_with_func(
		local_path: impl AsRef<std::path::Path>,
		item_fn: ItemFn,
	) -> Self {
		let path = local_path.as_ref();
		let method =
			HttpMethod::from_str(&item_fn.sig.ident.to_string()).unwrap();
		Self {
			mod_ident: syn::parse_quote! {file0},
			mod_import: ModImport::Inline,
			frontmatter: syn::parse_quote! {{}},
			item_fn,
			canonical_path: CanonicalPathBuf::new_unchecked(path),
			local_path: path.to_path_buf(),
			route_info: RouteInfo {
				path: RoutePath::from_file_path(path).unwrap(),
				method,
			},
		}
	}

	/// Whether this route was created from a file called `index.rs`, used by the
	/// [`RouteTreeBuilder`] to determine if it should be a child
	pub fn is_index(&self) -> bool {
		self.canonical_path
			.file_stem()
			.map(|s| s == "index")
			.unwrap_or(false)
	}
	/// "index" if the file stem ends with "index", otherwise the final part of the route path.
	/// This ensures the name reflects any route transformations.
	/// ## Panics
	/// If the route is not an index and the stem is not present, which should never happen.
	pub fn name(&self) -> String {
		if self.is_index() {
			return "index".to_string();
		} else {
			self.route_info
				.path
				.file_stem()
				.expect("File stem should always be present")
				.to_string_lossy()
				.to_string()
		}
	}

	/// Returns a path to the function, assuming its [`ModImport`] is in
	/// scope.
	pub fn func_path(&self) -> syn::Path {
		let mod_ident = &self.mod_ident;
		let fn_ident = &self.item_fn.sig.ident;
		syn::parse_quote! {
			#mod_ident::#fn_ident
		}
	}

	/// Return a `mod` import for each [`FuncTokens::func`]
	/// that requires a module import. The modules are public because
	/// client islands may need to call them.
	// this approach is cleaner than importing in each function,
	// and also rust-analyzer has an easier time resolving file level imports
	pub fn item_mod(&self, codegen_file: &CodegenFile) -> Result<ItemMod> {
		let mod_ident = &self.mod_ident;
		match self.mod_import {
			ModImport::Inline => {
				let item = &self.item_fn;
				Ok(syn::parse_quote! {
					pub mod #mod_ident {
						#[allow(unused_imports)]
						use super::*;
						#item
					}
				})
			}
			ModImport::Path => {
				let out_dir = codegen_file.output_dir()?;
				let mod_path =
					PathExt::create_relative(out_dir, &self.canonical_path)?;
				let mod_path_str = mod_path.to_string_lossy();
				Ok(syn::parse_quote! {
					#[path = #mod_path_str]
					pub mod #mod_ident;
				})
			}
		}
	}
}

/// The strategy to use for importing the module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModImport {
	/// The func doesnt actually exist yet (ie it was generated via markdown)
	/// so when defining the module include it as an item.
	Inline,
	/// The mod is imported from a rust file and its path will be
	/// resolved on codegen.
	Path,
}
