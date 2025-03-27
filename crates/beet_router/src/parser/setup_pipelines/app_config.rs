use crate::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// The app config is parsed by the cli at the first step of a build.
/// It provides essential information for the cli, including
/// the name of the package and details for codegen.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
	pub build_steps: Vec<SerdeBuildStep>,
	// temp, this will not scale
	/// Steps that should be ran after ExportStatic but before BuildWasm
	pub wasm_build_steps: Vec<SerdeBuildStep>,
}

impl AppConfig {
	/// Create a new Collection Builder.
	/// ## Panics
	/// Panics if the current working directory cannot be determined.
	pub fn new() -> Self {
		Self {
			build_steps: Vec::new(),
			wasm_build_steps: Vec::new(),
		}
	}

	pub fn add_step(mut self, group: impl Into<SerdeBuildStep>) -> Self {
		self.build_steps.push(group.into());
		self
	}

	pub fn add_wasm_step(mut self, group: impl Into<SerdeBuildStep>) -> Self {
		self.wasm_build_steps.push(group.into());
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
