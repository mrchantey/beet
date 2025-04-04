#[allow(unused_imports)]
use crate::prelude::*;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::Block;


/// An rsx function that has been parsed from some non-vanilla rust format
/// like markdown or mdrsx.
pub struct FuncTokens {
	/// A block that returns the frontmatter of this function.
	pub frontmatter: Block,
	/// A block that returns an RsxNode.
	pub block: Block,
	/// Canonical path to the file
	pub canonical_path: CanonicalPathBuf,
	/// Path relative to the [`src`](FileGroup::src) of the [`FileGroup`]
	pub local_path: PathBuf,
}



impl FuncTokens {}
