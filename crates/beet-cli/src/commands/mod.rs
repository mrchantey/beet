//! The individual `beet` CLI commands, each implemented as an action.

mod check;
mod export_pdf;
mod export_static;
#[cfg(feature = "qrcode")]
mod qrcode;
mod run_wasm;
mod s3_sync;
mod serve;

pub use check::*;
pub use export_pdf::*;
pub use export_static::*;
#[cfg(feature = "qrcode")]
pub use qrcode::*;
pub use run_wasm::*;
pub use s3_sync::*;
pub use serve::*;

use beet::prelude::*;

/// Registers reflection for every built-in `beet` command, so the default CLI
/// scene round-trips through `beet.json`: the exporter serializes the command
/// markers and the runner reconstructs their behaviour from the require hooks.
pub struct CliCommandsPlugin;

impl Plugin for CliCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Serve>()
			.register_type::<Check>()
			.register_type::<ExportStatic>()
			.register_type::<RunWasm>()
			.register_type::<BuildWasm>()
			.register_type::<ExportPdf>()
			.register_type::<SyncS3>();
		#[cfg(feature = "qrcode")]
		app.register_type::<QrCode>();
	}
}
