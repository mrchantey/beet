#[allow(unused_imports)]
use crate::prelude::*;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::Block;
use syn::Ident;


/// Tokens for a function that may be used as a route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncTokens {
	/// A unique identifier for this file based on its index in
	/// the [`FileGroup`], ie `file1`. It is used for importing the file
	/// as a module by its path. We need this awkwardness because rust analyzer
	/// cannot detect path imports nested inside a block.
	pub mod_ident: Option<Ident>,
	/// A block that returns the frontmatter of this function.
	pub frontmatter: Block,
	/// Tokens that will return a valid [`RouteFunc::func`], its exact signature depends
	/// on [`FuncTokensToCodegen::func_type`]. This may depend on [`mod_ident`](Self::mod_ident),
	/// to be imported and in scope.
	pub func: syn::Expr,
	/// Canonical path to the file
	pub canonical_path: CanonicalPathBuf,
	/// Path relative to the [`src`](FileGroup::src) of the [`FileGroup`]
	pub local_path: PathBuf,
	/// A reasonable route path generated from this file's local path,
	/// and a method matching either the functions signature, or
	/// `get` in the case of markdown.
	pub route_info: RouteInfo,
}



impl FuncTokens {
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
}
