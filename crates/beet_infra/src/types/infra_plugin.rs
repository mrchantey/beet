use beet_core::prelude::*;

/// The infra runtime + the deploy block/action type registrations, so adding
/// `InfraPlugin` makes every compiled deploy type spawnable by tag (eg
/// `<CloudflareWorkerBlock/>`, `<TofuApplyAction/>`) independent of the example
/// wiring. Each `register_type` is gated by the same feature as the type's
/// definition, so only the types actually compiled register.
///
/// The plugin itself is target-agnostic: the *definitions* (blocks, variables)
/// register everywhere, so a wasm consumer can author and serialize a stack, and
/// only the deploy actions — which shell out — are native-only.
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

		// the cloudflare deploy blocks, spawned by tag. Definitions, so every target.
		#[cfg(feature = "cloudflare_block")]
		app.register_type::<crate::prelude::CloudflareWorkerBlock>()
			.register_type::<crate::prelude::CloudflareContainerBlock>();

		// the cloudflare config components + the directly-spawnable cloudflare
		// deploy actions (`#[action(handler_only)]` + `#[reflect(Component,
		// Default)]`), all of which drive `wrangler` as a child process.
		#[cfg(all(feature = "cloudflare_block", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::CloudflareR2Sync>()
			.register_type::<crate::prelude::CloudflareBench>()
			.register_type::<crate::prelude::CloudflareWatch>()
			.register_type::<crate::prelude::CloudflareDestroy>()
			.register_type::<crate::prelude::CloudflareWorkerBuildAction>()
			.register_type::<crate::prelude::CloudflareWorkerDeployAction>()
			.register_type::<crate::prelude::CloudflareContainerDeployAction>();

		// the tofu apply action + the zone edge setup/purge (the whole `actions`
		// module is gated on `deploy`, and is native-only).
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::TofuApplyAction>()
			.register_type::<crate::prelude::CloudflareZoneSetup>()
			.register_type::<crate::prelude::CloudflarePurgeCache>();

		// the full-lifecycle smoke-test action: reads a bucket's `BlobStore` (so
		// `aws_sdk`-gated like the store) and lives in the `actions` module (so
		// `deploy`-gated and native-only). Register it only when all three compile it.
		#[cfg(all(
			feature = "deploy",
			feature = "aws_sdk",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::LifecycleProbe>();

		// the docker/podman image build action + its engine selector (the
		// `build_docker_image` module is gated on `fargate_block`).
		#[cfg(all(feature = "fargate_block", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::BuildDockerImage>()
			.register_type::<crate::prelude::ContainerEngine>();
	}
}
