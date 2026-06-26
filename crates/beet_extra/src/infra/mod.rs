//! The deploy example wiring for the `beet` binary.
//!
//! The infra examples (`examples/infra/hello_*.bsx`) run through the one binary,
//! eg `beet --main=examples/infra/hello_cloudflare_workers.bsx deploy`. This module
//! registers the deploy block/config types (so a scene's `<CloudflareWorkerBlock/>`
//! tag resolves from markup) and the standalone deploy actions as templates (their
//! `Request`/`Response` IO is not `Reflect`, so they cannot be reflect-spawned tags
//! directly).
pub mod infra_ext;
mod templates;
pub use templates::*;

use beet_core::prelude::*;
use beet_infra::prelude::*;

/// Adds the [`InfraPlugin`] runtime and registers the deploy example types, so a
/// loaded `examples/infra/*.bsx` scene resolves its `<CloudflareWorkerBlock/>` etc
/// tags and its `deploy`/`sync`/`destroy` routes run.
pub struct InfraExamplesPlugin;

impl Plugin for InfraExamplesPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<InfraPlugin>()
			// the cloudflare deploy blocks + config components, spawned by tag.
			.register_type::<CloudflareWorkerBlock>()
			.register_type::<CloudflareContainerBlock>()
			.register_type::<CloudflareR2Sync>()
			.register_type::<CloudflareWatch>()
			.register_type::<CloudflareDestroy>()
			// the deploy `Variable`, a field of the blocks' `env_vars`.
			.register_type::<Variable>()
			.register_type::<VariableValue>()
			// the standalone deploy actions, as templates (non-`Reflect` IO).
			.register_template::<CloudflareWorkerDeploy>()
			.register_template::<CloudflareContainerDeploy>()
			// the `beet` binary build artifact (zigbuild), for binary-shipping deploys.
			.register_template::<BeetBinaryBuild>()
			// the AWS deploy templates, wrapping the non-`Reflect` infra types so a
			// `.bsx` lambda deployer composes them (see `templates.rs`).
			.register_template::<StackHost>()
			.register_template::<SiteBucket>()
			.register_template::<TofuApply>()
			.register_template::<SiteSync>()
			.register_template::<LambdaSiteBlock>()
			.register_template::<LambdaWatch>()
			.register_template::<LightsailSiteBlock>()
			.register_template::<LightsailWatch>()
			.register_template::<FargateSiteBlock>()
			.register_template::<FargateWatch>()
			.register_template::<DockerImageBuild>();
	}
}
