use crate::prelude::*;
use anyhow::Result;
use std::path::PathBuf;
use sweet::prelude::CanonicalPathBuf;
use syn::Ident;

/// A definition of a file whose purpose is to expose functions
/// for parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncFile {
	/// A unique identifier for this file based on its index in
	/// the [`FileGroup`], ie `file1`. It is used for importing the file
	/// as a module by its path.
	pub ident: Ident,
	/// Canonical path to the file
	pub canonical_path: CanonicalPathBuf,
	/// Path relative to the [`FileGroup::src`]
	pub local_path: PathBuf,
	/// Tokens for the functions visited
	pub funcs: Vec<syn::Signature>,
}

impl FuncFile {
	/// Create a simple route path for this file, based on
	/// its [`FuncFile::local_path`].
	pub fn default_route_path(&self) -> Result<RoutePath> {
		RoutePath::parse_local_path(&self.local_path)
	}
}
