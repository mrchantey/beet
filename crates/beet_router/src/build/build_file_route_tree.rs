use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFileRouteTree {
	pub to_route_tree: FuncFilesToRouteTree,
	pub build_steps: Vec<BuildFileRoutes>,
}

impl BuildFileRouteTree {
	pub fn new(to_route_tree: FuncFilesToRouteTree) -> Self {
		Self {
			to_route_tree,
			build_steps: Vec::new(),
		}
	}
	pub fn with_step(mut self, step: BuildFileRoutes) -> Self {
		self.build_steps.push(step);
		self
	}
}

impl BuildStep for BuildFileRouteTree {
	// this is one of the most awkward build steps,
	// currently rsx piping really breaks down when it comes
	// to splitting and joining
	fn run(&self) -> Result<()> {
		self.build_steps
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
			.collect::<Vec<_>>()
			.pipe(self.to_route_tree.clone())
	}
}
