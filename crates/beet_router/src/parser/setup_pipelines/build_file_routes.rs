use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;

// const HTTP_METHODS: [&str; 9] = [
// 	"get", "post", "put", "delete", "head", "options", "connect", "trace",
// 	"patch",
// ];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFileRoutes {
	pub codegen_file: CodegenFile,
	pub file_group: FileGroup,
	pub group_to_funcs: FileGroupToFuncs,
	pub funcs_to_codegen: FileFuncsToCodegen,
}
impl BuildFileRoutes {
	pub fn new(
		src_dir: impl Into<WorkspacePathBuf>,
		out_file: impl Into<WorkspacePathBuf>,
		pkg_name: &str,
	) -> Self {
		let src_dir = src_dir.into().into_canonical_unchecked();
		let output = out_file.into().into_canonical_unchecked();

		Self {
			codegen_file: CodegenFile {
				output,
				pkg_name: Some(pkg_name.into()),
				..Default::default()
			},
			file_group: FileGroup::new(src_dir).with_filter(
				GlobFilter::default()
					.with_include("*.rs")
					.with_exclude("*mod.rs"),
			),
			group_to_funcs: FileGroupToFuncs::default(),
			funcs_to_codegen: FileFuncsToCodegen::default(),
		}
	}
	/// A common configuration of [`BuildComponentRoutes`] is to collect all mockup files in a directory.
	pub fn mockups(
		src_dir: impl Into<WorkspacePathBuf>,
		pkg_name: &str,
	) -> Self {
		let src_dir = src_dir.into().into_canonical_unchecked();
		let output =
			CanonicalPathBuf::new_unchecked(src_dir.join("codegen/mockups.rs"));

		Self {
			codegen_file: CodegenFile {
				output,
				pkg_name: Some(pkg_name.into()),
				..Default::default()
			},
			file_group: FileGroup::new(src_dir)
				.with_filter(GlobFilter::default().with_include("*.mockup.rs")),
			group_to_funcs: FileGroupToFuncs {
				route_path_prefix: Some("/mockups".into()),
				route_path_replace: vec![(
					".mockup".into(),
					Default::default(),
				)],
				..Default::default()
			},
			funcs_to_codegen: FileFuncsToCodegen::default(),
		}
	}
}

/// this is usually not called, instead we use [`BuildFileRouteTree`]
/// which reduces duplication of work
impl BuildStep for BuildFileRoutes {
	fn run(&self) -> Result<()> {
		let Self {
			codegen_file,
			file_group,
			group_to_funcs,
			funcs_to_codegen,
		} = self.clone();
		file_group
			.pipe(group_to_funcs)?
			.pipe_with(codegen_file, funcs_to_codegen)?
			.build_and_write()?;
		Ok(())
	}
}
