//! The individual `beet` CLI commands, each implemented as an action.

mod analytics;
mod check;
mod entry;
mod export_pdf;
mod export_static;
#[cfg(feature = "pdf")]
pub mod pdf_ext;
#[cfg(feature = "qrcode")]
mod qrcode;
mod run_wasm;
#[cfg(not(target_arch = "wasm32"))]
mod run_wasm_browser;
mod s3_sync;
#[cfg(not(target_arch = "wasm32"))]
mod screenshot;
// the committed page-driving check for the browser render boot, run via
// `just check-wasm-render`.
#[cfg(test)]
mod wasm_render_check;

pub use analytics::*;
pub use check::*;
pub(crate) use entry::*;
pub use export_pdf::*;
pub use export_static::*;
#[cfg(feature = "qrcode")]
pub use qrcode::*;
pub use run_wasm::*;
pub use s3_sync::*;
#[cfg(not(target_arch = "wasm32"))]
pub use screenshot::*;

use beet::prelude::*;

/// Registers reflection for every `beet` dev command, so a `main.bsx` can name
/// them as route actions. The binary spawns no host; these are inert capabilities
/// until an entry wires them.
pub struct CliCommandsPlugin;

impl Plugin for CliCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<AnalyticsReport>()
			.register_type::<Check>()
			.register_type::<ExportStatic>()
			.register_type::<RunWasm>()
			.register_type::<BuildWasm>()
			.register_type::<BuildWasmAction>()
			.register_type::<ExportPdf>()
			.register_type::<SyncS3>();
		#[cfg(not(target_arch = "wasm32"))]
		app.register_type::<CaptureScreenshot>();
		#[cfg(feature = "qrcode")]
		app.register_type::<QrCode>();
		// NOTE: deploy tags are NOT allowlisted here. An entry declaring deploy
		// verbs gates them with `bx:features="infra,extra"`, so a build without
		// those features skips the subtree at resolve time instead of relying on
		// a hand-maintained list of tags to treat as inert — a list that silently
		// went stale every time an entry named a new one.
	}
}
