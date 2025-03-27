use crate::parser::FileGroup;
use anyhow::Result;
use beet_rsx::rsx::RsxPipeline;
use beet_rsx::rsx::RsxPipelineTarget;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use sweet::prelude::CanonicalPathBuf;
use sweet::prelude::ReadFile;
use sweet::prelude::*;
use syn::Visibility;


/// For a given file group, collect all public functions.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileGroupToFuncs {}

impl FileGroupToFuncs {
	fn build_file_funcs(
		&self,
		group_src: &CanonicalPathBuf,
		file: PathBuf,
	) -> Result<FileFuncs> {
		let file_str = ReadFile::to_string(&file)?;
		let funcs = syn::parse_file(&file_str)?
			.items
			.into_iter()
			.filter_map(|item| {
				if let syn::Item::Fn(f) = item {
					match &f.vis {
						Visibility::Public(_) => {
							return Some(f.sig);
						}
						_ => {}
					}
				}
				None
			})
			.collect::<Vec<_>>();

		let canonical_path = CanonicalPathBuf::new(file)?;
		let local_path = PathExt::create_relative(&group_src, &canonical_path)?;
		let route = RoutePath::parse(&local_path)?;

		Ok(FileFuncs {
			canonical_path,
			local_path,
			route,
			funcs,
		})
	}
}

impl RsxPipeline<FileGroup, Result<Vec<FileFuncs>>> for FileGroupToFuncs {
	fn apply(self, group: FileGroup) -> Result<Vec<FileFuncs>> {
		group
			.collect_files()?
			.into_iter()
			.map(|p| self.build_file_funcs(&group.src, p))
			.collect::<Result<Vec<_>>>()
	}
}

#[derive(Debug, Clone)]
/// Collection of all public functions in a file
pub struct FileFuncs {
	/// Canonical path to the file
	pub canonical_path: CanonicalPathBuf,
	/// Path relative to the [`FileGroup::src`]
	pub local_path: PathBuf,
	/// Route for the file
	pub route: RoutePath,
	/// Tokens for the functions visited
	pub funcs: Vec<syn::Signature>,
}

impl FileFuncs {}

impl RsxPipelineTarget for FileFuncs {}


#[derive(Debug, Clone)]
pub struct RoutePath(PathBuf);

impl std::ops::Deref for RoutePath {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl Into<PathBuf> for RoutePath {
	fn into(self) -> PathBuf { self.0 }
}

impl RoutePath {
	pub fn new(path: impl Into<PathBuf>) -> Self { Self(path.into()) }
	pub fn parse(local_path: &Path) -> Result<Self> {
		let mut raw_str = local_path
			.to_string_lossy()
			.replace(".rs", "")
			.replace("\\", "/");
		if raw_str.ends_with("index") {
			raw_str = raw_str.replace("index", "");
			// remove trailing `/` from non root paths
			if raw_str.len() > 1 {
				raw_str.pop();
			}
		};
		raw_str = format!("/{}", raw_str);

		Ok(Self(PathBuf::from(raw_str)))
	}
}


#[cfg(test)]
mod test {
	use std::path::Path;

	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let funcs = FileGroup::test_site_routes()
			.pipe(FileGroupToFuncs::default())
			.unwrap();
		expect(funcs.len()).to_be(3);
		let docs = funcs
			.iter()
			.find(|f| &*f.route == Path::new("/docs"))
			.unwrap();
		expect(docs.funcs.len()).to_be(1);
		expect(&docs.local_path.to_string_lossy()).to_be("docs/index.rs");
		expect(docs.canonical_path.to_string_lossy().ends_with(
			"crates/beet_router/src/test_site/routes/docs/index.rs",
		))
		.to_be_true();
	}
}
