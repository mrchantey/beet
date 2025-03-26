use crate::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// The app config is parsed by the cli at the first step of a build.
/// It provides essential information for the cli, including
/// the name of the package and details for codegen.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
	pub codegen_steps: Vec<CodegenStep>,
}

impl AppConfig {
	/// Create a new Collection Builder.
	/// ## Panics
	/// Panics if the current working directory cannot be determined.
	pub fn new() -> Self {
		Self {
			codegen_steps: Vec::new(),
		}
	}

	pub fn add_step(mut self, group: impl Into<CodegenStep>) -> Self {
		self.codegen_steps.push(group.into());
		self
	}

	/// Serializes self and writes to stdout, which is collected by the beet cli.
	///
	/// ## Panics
	/// Panics if serialization fails.
	pub fn export(&self) {
		let ron = ron::ser::to_string_pretty(self, Default::default())
			.expect("failed to serialize");
		println!("{}", ron);
	}
}


#[derive(Debug, Serialize, Deserialize)]
pub enum CodegenStep {
	FileRoutes(BuildFileRoutes),
	FileComponents(BuildFileComponents),
}


impl Into<CodegenStep> for BuildFileRoutes {
	fn into(self) -> CodegenStep { CodegenStep::FileRoutes(self) }
}

impl Into<CodegenStep> for BuildFileComponents {
	fn into(self) -> CodegenStep { CodegenStep::FileComponents(self) }
}
