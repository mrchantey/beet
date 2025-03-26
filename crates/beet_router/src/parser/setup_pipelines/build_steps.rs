use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerdeBuildStep {
	FileRoutes(BuildFileRoutes),
	ComponentRoutes(BuildComponentRoutes),
}


impl Into<SerdeBuildStep> for BuildFileRoutes {
	fn into(self) -> SerdeBuildStep { SerdeBuildStep::FileRoutes(self) }
}

impl Into<SerdeBuildStep> for BuildComponentRoutes {
	fn into(self) -> SerdeBuildStep { SerdeBuildStep::ComponentRoutes(self) }
}

impl BuildStep for SerdeBuildStep {
	fn run(&self) -> Result<()> {
		match self {
			SerdeBuildStep::FileRoutes(step) => step.run(),
			SerdeBuildStep::ComponentRoutes(step) => step.run(),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildComponentRoutes {
	codegen_file: CodegenFile,
	file_group: FileGroup,
	group_to_funcs: FileGroupToFuncs,
	funcs_to_codegen: FileFuncsToCodegen,
}



impl BuildComponentRoutes {
	/// A common configuration of [`BuildComponentRoutes`] is to collect all mockup files in a directory.
	pub fn mockups(
		src_dir: impl Into<WorkspacePathBuf>,
		pkg_name: &str,
	) -> Self {
		let src_dir = src_dir.into();
		let output =
			CanonicalPathBuf::new_unchecked(src_dir.join("codegen/mockups.rs"));

		Self {
			codegen_file: CodegenFile {
				output,
				pkg_name: Some(pkg_name.into()),
				..Default::default()
			},
			file_group: FileGroup::new_workspace_rel(src_dir)
				.unwrap()
				.with_filter(GlobFilter::default().with_include("*.mockup.rs")),
			group_to_funcs: FileGroupToFuncs::default(),
			funcs_to_codegen: FileFuncsToCodegen::default(),
		}
	}
}

impl BuildStep for BuildComponentRoutes {
	#[rustfmt::skip]
	fn run(&self) -> Result<()> {
		let Self {
			codegen_file,
			file_group,
			group_to_funcs,
			funcs_to_codegen,
		} = self.clone();
		file_group
			.pipe(group_to_funcs)?
			.pipe_with(codegen_file,funcs_to_codegen)?
			.build_and_write()?;
		Ok(())
	}
}



const HTTP_METHODS: [&str; 9] = [
	"get", "post", "put", "delete", "head", "options", "connect", "trace",
	"patch",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFileRoutes {
	codegen_file: CodegenFile,
	file_group: FileGroup,
	group_to_funcs: FileGroupToFuncs,
	funcs_to_codegen: FileFuncsToCodegen,
}
impl BuildFileRoutes {
	pub fn new(src_dir: impl Into<WorkspacePathBuf>, pkg_name: &str) -> Self {
		let src_dir = src_dir.into();
		let output =
			CanonicalPathBuf::new_unchecked(src_dir.join("codegen/routes.rs"));

		Self {
			codegen_file: CodegenFile {
				output,
				pkg_name: Some(pkg_name.into()),
				..Default::default()
			},
			file_group: FileGroup::new_workspace_rel(src_dir)
				.unwrap()
				.with_filter(
					GlobFilter::default()
						.with_include("*.rs")
						.with_exclude("*mod.rs"),
				),
			group_to_funcs: FileGroupToFuncs::default(),
			funcs_to_codegen: FileFuncsToCodegen::default(),
		}
	}
}

impl BuildStep for BuildFileRoutes {
	#[rustfmt::skip]
	fn run(&self) -> Result<()> {
		let Self {
			codegen_file,
			file_group,
			group_to_funcs,
			funcs_to_codegen,
		} = self.clone();
		file_group
			.pipe(group_to_funcs)?
			.pipe_with(codegen_file,funcs_to_codegen)?
			.build_and_write()?;
		Ok(())
	}
}
