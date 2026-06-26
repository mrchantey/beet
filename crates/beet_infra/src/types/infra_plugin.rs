use beet_core::prelude::*;

/// The infra runtime + the deploy block/action type registrations, so adding
/// `InfraPlugin` makes every compiled deploy type spawnable by tag (eg
/// `<CloudflareWorkerBlock/>`, `<TofuApplyAction/>`) independent of the example
/// wiring. Each `register_type` is gated by the same feature as the type's
/// definition, so only the types actually compiled register.
#[derive(Default)]
pub struct InfraPlugin;

impl Plugin for InfraPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>();
		#[cfg(feature = "deploy")]
		app.init_plugin::<beet_router::prelude::RouterPlugin>();

		// the deploy `Variable` + its value resolution, a field of the blocks'
		// `env_vars` (always compiled, in `types/`).
		app.register_type::<crate::types::Variable>()
			.register_type::<crate::types::VariableValue>();

		// the cloudflare deploy blocks + config components, spawned by tag.
		#[cfg(feature = "cloudflare_block")]
		app.register_type::<crate::prelude::CloudflareWorkerBlock>()
			.register_type::<crate::prelude::CloudflareContainerBlock>()
			.register_type::<crate::prelude::CloudflareR2Sync>()
			.register_type::<crate::prelude::CloudflareWatch>()
			.register_type::<crate::prelude::CloudflareDestroy>()
			// the directly-spawnable cloudflare deploy actions (`#[action(handler_only)]`
			// + `#[reflect(Component, Default)]`).
			.register_type::<crate::prelude::CloudflareWorkerDeployAction>()
			.register_type::<crate::prelude::CloudflareContainerDeployAction>();

		// the tofu apply action (the whole `actions` module is gated on `deploy`).
		#[cfg(feature = "deploy")]
		app.register_type::<crate::prelude::TofuApplyAction>();

		// the docker/podman image build action + its engine selector (the
		// `build_docker_image` module is gated on `fargate_block`).
		#[cfg(feature = "fargate_block")]
		app.register_type::<crate::prelude::BuildDockerImage>()
			.register_type::<crate::prelude::ContainerEngine>();
	}
}
