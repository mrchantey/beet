use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::Expr;
use syn::Ident;
use syn::ItemMod;
use syn::Signature;
use syn::Type;


/// The output of [`FuncFilesToRouteFuncs`], represents a mapping of a
/// [`FileGroup`] to a [`Vec<RouteFunc`], we need to keep tracking each
/// [`FuncFile`] for codgen to import the modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteFuncGroup {
	pub func_files: Vec<FuncFile>,
	/// The created route functions
	pub route_funcs: Vec<RouteFuncTokens>,
}


/// Intermediate representation for building a [`RouteFunc`]. This must be kept
/// associated with the [`FuncFile`] that it was created from, as the [`RouteFuncTokens::block`]
/// that has been created will depend on its import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteFuncTokens {
	/// The best name for this file. If it is an index file, it will be "index",
	/// otherwise it will be the last segment of the `route_path`.
	/// This is to respect any transformations done in the `FileGroupToFuncs`,
	/// ie removing a `.mockup` suffix
	pub name: String,
	/// Canonical path to the file
	pub canonical_path: CanonicalPathBuf,
	/// Path relative to the [`FileGroup::src`]
	pub local_path: PathBuf,
	/// Route for the file
	pub route_path: RoutePath,
	/// A block that will return a valid function, its exact signature depends
	/// on [`FuncFilesToRouteFuncs::func_type`]. This block will contain references
	/// to the corresponding [`FuncFile`] ident, by its index, ie `file0::get`, so
	/// its imperative that this block is used in a codegen file with its corresponding
	/// [`FuncFile`] mod import.
	pub block: syn::Block,
}


impl RsxPipelineTarget for RouteFuncGroup {}


/// Convert a vec of [`FuncFile`] into a vec of [`RouteFuncTokens`].
/// The number of functions is usally different, ie file based routes may
/// have a `get` and `post` function, whereas mockups may merge all
/// functions into one route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuncFilesToRouteFuncs {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FuncFilesToRouteFuncsStrategy {
	/// Each function will be mapped to a corresponding http method
	Http,
}

impl Default for FuncFilesToRouteFuncs {
	fn default() -> Self { Self {} }
}


impl RsxPipeline<Vec<FuncFile>, Result<RouteFuncGroup>>
	for FuncFilesToRouteFuncs
{
	fn apply(self, func_files: Vec<FuncFile>) -> Result<RouteFuncGroup> {
		let route_funcs = func_files
			.iter()
			.map(|sigs| self.file_func_to_route_funcs(sigs))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();
		Ok(RouteFuncGroup {
			route_funcs,
			func_files,
		})
	}
}


impl FuncFilesToRouteFuncs {
	pub fn file_func_to_route_funcs(
		&self,
		_func: &FuncFile,
	) -> Result<Vec<RouteFuncTokens>> {
		Ok(vec![])
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	// use quote::ToTokens;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let _route_funcs = FileGroup::test_site_routes()
			.pipe(FileGroupToFuncs::default())
			.unwrap()
			.pipe(FuncFilesToRouteFuncs::default())
			.unwrap();
	}
}
