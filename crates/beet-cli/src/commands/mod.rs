//! The individual `beet` CLI commands, each implemented as an action.

mod export_pdf;
#[cfg(feature = "qrcode")]
mod qrcode;
mod run_wasm;

pub use export_pdf::*;
#[cfg(feature = "qrcode")]
pub use qrcode::*;
pub use run_wasm::*;

use beet::prelude::*;

/// Registers reflection for every built-in `beet` command, so the default CLI
/// scene round-trips through `beet.json`: the exporter serializes the command
/// markers and the runner reconstructs their behaviour from the require hooks.
pub struct CliCommandsPlugin;

impl Plugin for CliCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<RunWasm>()
			.register_type::<BuildWasm>()
			.register_type::<ExportPdf>();
		#[cfg(feature = "qrcode")]
		app.register_type::<QrCode>();
	}
}
