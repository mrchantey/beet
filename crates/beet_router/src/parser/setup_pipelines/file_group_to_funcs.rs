use crate::parser::FileGroup;
use crate::parser::RoutePath;
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
pub struct FileGroupToFuncs {
	/// prepend the route path
	pub route_path_prefix: Option<RoutePath>,
	/// Replace parts of the route path with others
	pub route_path_replace: Vec<(String, String)>,
}

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
		let route_path = self.build_route_path(&local_path)?;
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


		Ok(FileFuncs {
			name,
			canonical_path,
			local_path,
			route_path,
			funcs,
		})
	}

	fn build_route_path(&self, local_path: &Path) -> Result<RoutePath> {
		let path = RoutePath::parse_local_path(&local_path)?;
		let mut path_str = path.to_string_lossy().to_string();

		for (from, to) in self.route_path_replace.iter() {
			path_str = path_str.replace(from.as_str(), to.as_str());
		}
		let mut path = RoutePath::new(path_str);
		if let Some(prefix) = &self.route_path_prefix {
			path = prefix.join(&path);
		}

		Ok(path)
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

#[derive(Debug, Clone, PartialEq, Eq)]
/// Collection of all public functions in a file
pub struct FileFuncs {
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
	/// Tokens for the functions visited
	pub funcs: Vec<syn::Signature>,
}

impl FileFuncs {}

impl RsxPipelineTarget for FileFuncs {}


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
			.find(|f| &*f.route_path == Path::new("/docs"))
			.unwrap();
		expect(docs.funcs.len()).to_be(1);
		expect(&docs.local_path.to_string_lossy()).to_be("docs/index.rs");
		expect(docs.canonical_path.to_string_lossy().ends_with(
			"crates/beet_router/src/test_site/routes/docs/index.rs",
		))
		.to_be_true();
	}
}
