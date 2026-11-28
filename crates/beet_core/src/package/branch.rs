//! The identity of a branch and what a branch holds: [`Branch`],
//! [`SourceBranch`] and [`Files`].
use crate::prelude::*;
use alloc::collections::BTreeMap;

/// `org.beet.core#branch`, the identity of a branch record and of the package
/// record compiled from it, rkey `<package>:<branch>`, ie `todos:main`.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, LexiconDef, Serialize, Deserialize)]
pub struct Branch {
	/// The package.
	pub package: PackageName,
	/// The branch.
	pub name: BranchName,
}

impl Branch {
	/// The branch `name` of `package`.
	pub fn new(package: PackageName, name: BranchName) -> Self {
		Self { package, name }
	}

	/// The rkey a branch record and its package record live at.
	pub fn rkey(&self) -> Rkey {
		Rkey::parse(&format!("{}:{}", self.package, self.name))
			.expect("a package and branch name are rkey safe")
	}
}

/// `org.beet.core#sourceBranch`, on a package record and kept by its releases:
/// the branch record it was compiled from, as it was. A staleness check, never
/// a hop a reader follows: the package record is stale exactly when the
/// branch's current cid is no longer this one.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, LexiconDef, Serialize, Deserialize)]
pub struct SourceBranch(pub StrongRef);

impl SourceBranch {
	/// Whether the branch record at `current` is not the one compiled.
	pub fn is_stale(&self, current: &Cid) -> bool { &self.0.cid != current }
}

/// `org.beet.core#files`, on a branch record, a package record and a release:
/// every file of the package by its path, extension kept, so `Card.bsx` and
/// `Card.md` are two files, each naming its blob. A file whose media type has
/// a parser is a document and compiles into a scene; any other is loaded as
/// its blob. Holding a [`BlobRef`] is what keeps each retained.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, Default, LexiconDef, Serialize, Deserialize)]
pub struct Files(pub BTreeMap<FwdPath, BlobRef>);

impl Files {
	/// Every path that names `path` with its extension left off, ie
	/// `layouts/Article.bsx` for `layouts/Article`: the file itself where it
	/// exists, else every file of that stem.
	pub fn completions(&self, path: &FwdPath) -> Vec<FwdPath> {
		if self.0.contains_key(path) {
			return vec![path.clone()];
		}
		let prefix = format!("{path}.");
		self.0
			.keys()
			.filter(|candidate| {
				candidate
					.as_str()
					.strip_prefix(prefix.as_str())
					.is_some_and(|extension| !extension.contains('/'))
			})
			.cloned()
			.collect()
	}
}
