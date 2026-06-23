//! The individual `beet` CLI commands, each implemented as an action.

mod check;
mod export_pdf;
mod export_static;
#[cfg(feature = "qrcode")]
mod qrcode;
mod run_wasm;
mod s3_sync;
mod serve;
mod site;

pub use check::*;
pub use export_pdf::*;
pub use export_static::*;
#[cfg(feature = "qrcode")]
pub use qrcode::*;
pub use run_wasm::*;
pub use s3_sync::*;
pub use serve::*;
pub(crate) use site::*;

use beet::prelude::*;

/// Registers reflection for every `beet` dev command, so a `main.bsx` can name
/// them as route actions. The binary spawns no host; these are inert capabilities
/// until an entry wires them.
pub struct CliCommandsPlugin;

impl Plugin for CliCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Check>()
			.register_type::<ExportStatic>()
			// `serve <site>` loads a site and boots its servers (the only command
			// that boots the workspace entry's server, via the exchange->boot bridge)
			.register_type::<Serve>()
			.register_type::<RunWasm>()
			.register_type::<BuildWasm>()
			.register_type::<ExportPdf>()
			.register_type::<SyncS3>();
		#[cfg(feature = "qrcode")]
		app.register_type::<QrCode>();
	}
}
