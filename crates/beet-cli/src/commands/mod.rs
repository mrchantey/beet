//! The individual `beet` CLI commands, each implemented as an action.

mod analytics;
mod check;
mod export_pdf;
mod export_static;
#[cfg(feature = "pdf")]
pub mod pdf_ext;
#[cfg(feature = "qrcode")]
mod qrcode;
mod run_wasm;
mod s3_sync;
mod serve;
mod site;

pub use analytics::*;
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
		app.register_type::<AnalyticsReport>()
			.register_type::<Check>()
			.register_type::<ExportStatic>()
			// `serve <site>` loads a site and boots its servers (the only command
			// that boots the workspace entry's server, via a direct boot call)
			.register_type::<Serve>()
			.register_type::<RunWasm>()
			.register_type::<BuildWasm>()
			.register_type::<ExportPdf>()
			.register_type::<SyncS3>();
		#[cfg(feature = "qrcode")]
		app.register_type::<QrCode>();
		// the root `main.bsx` names the infra deploy blocks, which only register
		// under the `infra` feature. Without it (the default `beet`, `beet run-wasm`,
		// `beet build-wasm`, `beet serve`), allow them as known-but-inert tags so the
		// entry still loads and the `deploy`/`sync`/`watch` routes render nothing,
		// mirroring how `<LiveReloadScript>` degrades without `client_io`.
		#[cfg(not(feature = "infra"))]
		for tag in [
			"BeetSiteDeployHost",
			"FargateBeetSiteBlock",
			"CloudflareZoneSetup",
			"CloudflarePurgeCache",
			"SiteBucket",
			"AssetsBucket",
			"BeetBinaryBuild",
			"TofuApplyAction",
			"BuildDockerImage",
			"DirSync",
			"FargateWatch",
		] {
			app.allow_unregistered(tag);
		}
	}
}
