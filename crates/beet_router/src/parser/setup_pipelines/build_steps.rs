use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerdeBuildStep {
	FileRoutes(BuildFileRoutes),
	ComponentRoutes(BuildComponentRoutes),
	WasmRoutes(BuildWasmRoutes),
}


impl Into<SerdeBuildStep> for BuildFileRoutes {
	fn into(self) -> SerdeBuildStep { SerdeBuildStep::FileRoutes(self) }
}

impl Into<SerdeBuildStep> for BuildComponentRoutes {
	fn into(self) -> SerdeBuildStep { SerdeBuildStep::ComponentRoutes(self) }
}

impl Into<SerdeBuildStep> for BuildWasmRoutes {
	fn into(self) -> SerdeBuildStep { SerdeBuildStep::WasmRoutes(self) }
}

impl BuildStep for SerdeBuildStep {
	fn run(&self) -> Result<()> {
		match self {
			SerdeBuildStep::FileRoutes(step) => step.run(),
			SerdeBuildStep::ComponentRoutes(step) => step.run(),
			SerdeBuildStep::WasmRoutes(step) => step.run(),
		}
	}
}
