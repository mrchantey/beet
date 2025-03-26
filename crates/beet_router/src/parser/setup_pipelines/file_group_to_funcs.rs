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
		let route = Route::parse(&local_path)?;

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


/// Collection of all public functions in a file
pub struct FileFuncs {
	/// Canonical path to the file
	pub canonical_path: CanonicalPathBuf,
	/// Path relative to the [`FileGroup::src`]
	pub local_path: PathBuf,
	/// Route for the file
	pub route: Route,
	/// Tokens for the functions visited
	pub funcs: Vec<syn::Signature>,
}

impl FileFuncs {
	pub fn route(&self) -> String {
		let str = self.local_path.to_string_lossy();
		let str = str.replace(".rs", "").replace("\\", "/");
		if str.ends_with("index") {
			str.replace("index", "")
		} else {
			str
		}
	}
}

impl RsxPipelineTarget for FileFuncs {}


pub struct Route {
	pub raw_str: String,
}

impl Route {
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
		Ok(Self { raw_str })
	}
}
