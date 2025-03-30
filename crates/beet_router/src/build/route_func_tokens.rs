use crate::prelude::*;
use anyhow::Result;
use std::path::Path;
use syn::Block;

/// Intermediate representation for building a [`RouteFunc`]. This must be kept
/// associated with the [`FuncFile`] that it was created from, as the [`RouteFuncTokens::block`]
/// that has been created will depend on its import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteFuncTokens {
	/// The best name for this file. If it is an index file, it will be "index",
	/// otherwise it will be the last segment of the `route_path`.
	pub name: String,
	/// Whether this route was created from a file called `index.rs`, used by the
	/// [`RouteTreeBuilder`] to determine if it should be a child
	pub is_index: bool,
	/// Route for the file
	pub route_path: RoutePath,
	/// A block that will return a valid function, its exact signature depends
	/// on [`FuncFilesToRouteFuncs::func_type`]. This block will contain references
	/// to the corresponding [`FuncFile`] ident, by its index, ie `file0::get`, so
	/// its imperative that this block is used in a codegen file with its corresponding
	/// [`FuncFile`] mod import.
	pub block: syn::Block,
}

impl RouteFuncTokens {
	pub fn build(
		local_path: &Path,
		route_path: RoutePath,
		block: Block,
	) -> Result<Self> {
		let is_index = local_path
			.file_stem()
			.map(|s| s == "index")
			.unwrap_or(false);
		let name = if is_index {
			"index".to_string()
		} else {
			route_path
				.inner()
				.file_stem()
				.unwrap()
				.to_string_lossy()
				.to_string()
		};
		Ok(Self {
			name,
			is_index,
			route_path,
			block,
		})
	}
}
