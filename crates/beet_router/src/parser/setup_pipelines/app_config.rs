use crate::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// The app config is parsed by the cli at the first step of a build.
/// It provides essential information for the cli, including
/// the name of the package and details for codegen.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
	pub native_build_step: BuildFileRouteTree,
	// temp, this will not scale
	/// Steps that should be ran after ExportStatic but before BuildWasm
	pub wasm_build_step: BuildWasmRoutes,
}

impl AppConfig {
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
