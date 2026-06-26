//! The deploy example wiring for the `beet` binary.
//!
//! The infra examples (`examples/infra/hello_*.bsx`) run through the one binary,
//! eg `beet --main=examples/infra/hello_cloudflare_workers.bsx deploy`. The deploy
//! block/config types and directly-spawnable deploy actions register upstream in
//! beet_infra's [`InfraPlugin`]; this module adds that plugin plus the few templates
//! that wrap non-`Reflect` infra values (see `templates.rs`), so a scene's
//! `<StackHost>`/`<LambdaSiteBlock/>` tag resolves from markup.
pub mod infra_ext;
mod templates;
pub use templates::*;

use beet_core::prelude::*;
use beet_infra::prelude::*;

/// Adds the [`InfraPlugin`] runtime (which registers the deploy block/action types)
/// and registers the beet_extra deploy templates, so a loaded `examples/infra/*.bsx`
/// scene resolves its `<StackHost>` etc tags and its `deploy`/`sync`/`destroy`
/// routes run.
pub struct InfraExamplesPlugin;

impl Plugin for InfraExamplesPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<InfraPlugin>()
			// the `beet` binary build artifact (zigbuild), for binary-shipping deploys.
			.register_template::<BeetBinaryBuild>()
			// the AWS deploy templates, wrapping the non-`Reflect` infra types so a
			// `.bsx` lambda deployer composes them (see `templates.rs`).
			.register_template::<StackHost>()
			.register_template::<SiteBucket>()
			.register_template::<SiteSync>()
			.register_template::<LambdaSiteBlock>()
			.register_template::<LambdaWatch>()
			.register_template::<LightsailSiteBlock>()
			.register_template::<LightsailWatch>()
			.register_template::<FargateSiteBlock>()
			.register_template::<FargateWatch>();
	}
}
