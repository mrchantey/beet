//! The deploy example wiring for the `beet` binary.
//!
//! The infra examples (`examples/infra/hello_*.bsx`) run through the one binary,
//! eg `beet --entry=examples/infra/cloudflare_workers.bsx deploy`. The deploy
//! block/config types and directly-spawnable deploy actions register upstream in
//! beet_infra's [`InfraPlugin`]; this module adds that plugin plus the few templates
//! that wrap non-`Reflect` infra values (see `templates.rs`), so a scene's
//! `<DeployRoutes/>`/`<LambdaSiteBlock/>` tag resolves from markup.
pub mod infra_ext;
mod templates;
pub use templates::*;

use beet_core::prelude::*;
use beet_infra::prelude::*;

/// Adds the [`InfraPlugin`] runtime (which registers the deploy block/action types)
/// and registers the beet_extra deploy templates, so a loaded `examples/infra/*.bsx`
/// scene resolves its `<DeployRoutes/>` etc tags and its `deploy`/`sync`/`destroy`
/// routes run.
pub struct InfraExamplesPlugin;

impl Plugin for InfraExamplesPlugin {
	fn build(&self, app: &mut App) {
		app.install_type::<(
			InfraPlugin,
			// the `beet` binary build artifact (zigbuild), for binary-shipping deploys.
			BeetBinaryBuild,
			// an example-target binary build artifact (eg the `ssh_tui_site` server).
			ExampleBinaryBuild,
		)>()
		// the state-backend toggle middleware the bucket and handle examples
		// put on their router. The IaC verb routes register upstream in
		// `InfraPlugin`, beside the verbs.
		.register_type::<StateBackendToggle>()
		// the AWS deploy templates, wrapping the non-`Reflect` infra types so a
		// `.bsx` deploy entry composes them (see `templates.rs`): the
		// beet-site entry's lightsail block, stage-aware through the
		// `<Stack>` it resolves by ancestry. The resource declarations
		// themselves are authored as their blocks (`<S3BucketBlock/>`,
		// `<DynamoTableBlock/>`), registered upstream.
		.install_type::<(
			LightsailBeetSiteBlock,
			LambdaSiteBlock,
			// the invoke-only counterpart, the target a `<ScheduledJobBlock/>`
			// drives on a timer.
			LambdaJobBlock,
			LambdaWatch,
			LightsailSiteBlock,
			LightsailWatch,
			FargateSiteBlock,
			FargateSshBlock,
			FargateWatch,
		)>();
	}
}
