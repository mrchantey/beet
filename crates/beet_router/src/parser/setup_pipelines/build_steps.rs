use crate::prelude::*;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;
use anyhow::Result;


#[derive(Debug, Serialize, Deserialize)]
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
	file_group: FileGroup,
	group_to_funcs: FileGroupToFuncs,
	funcs_to_codegen: FileFuncsToCodegen,
}



impl BuildComponentRoutes {
	/// A common configuration of [`BuildComponentRoutes`] is to collect all mockup files in a directory.
	pub fn mockups(src_dir: impl Into<WorkspacePathBuf>) -> Self {
		let src_dir = src_dir.into();
		let output = src_dir.join("codegen/mockups.rs");

		Self {
			file_group: FileGroup::new_workspace_rel(src_dir)
				.unwrap()
				.with_filter(GlobFilter::default().with_include("*.mockup.rs")),
			group_to_funcs: FileGroupToFuncs::default(),
			funcs_to_codegen: FileFuncsToCodegen {
				output: output.into(),
				..Default::default()
			},
		}
	}
}

impl BuildStep for BuildComponentRoutes {
	#[rustfmt::skip]
	fn run(&self) -> Result<()> {
		let Self {
			file_group,
			group_to_funcs,
			funcs_to_codegen,
		} = self.clone();
		file_group
			.pipe(group_to_funcs)?
			.pipe(funcs_to_codegen)?;
		Ok(())
	}
}

pub struct BuildFileRoutes2 {}
