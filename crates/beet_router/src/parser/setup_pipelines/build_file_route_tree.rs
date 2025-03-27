use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFileRouteTree {
	pub codegen_file: CodegenFile,
	pub build_steps: Vec<BuildFileRoutes>,
}

impl BuildFileRouteTree {
	pub fn new(out_file: impl Into<WorkspacePathBuf>, pkg_name: &str) -> Self {
		let output = out_file.into().into_canonical_unchecked();
		Self {
			codegen_file: CodegenFile {
				output,
				pkg_name: Some(pkg_name.into()),
				..Default::default()
			},
			build_steps: Vec::new(),
		}
	}
	pub fn with_step(mut self, step: BuildFileRoutes) -> Self {
		self.build_steps.push(step);
		self
	}
}

impl BuildStep for BuildFileRouteTree {
	fn run(&self) -> Result<()> {
		let files = self
			.build_steps
			.iter()
			.map(|step| {
				let BuildFileRoutes {
					codegen_file,
					file_group,
					group_to_funcs,
					funcs_to_codegen,
				} = step.clone();
				let funcs = file_group.pipe(group_to_funcs)?;
				funcs
					.clone()
					.pipe_with(codegen_file, funcs_to_codegen)?
					.build_and_write()?;
				Ok(funcs)
			})
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();

		let route_tree = RouteTree::new(files.iter()).into_paths_mod();

		let mut codegen_file = self.codegen_file.clone();
		codegen_file.add_item(route_tree);
		codegen_file.build_and_write()?;

		Ok(())
	}
}
