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

		// the identity every declaration composes its name from, registered in
		// every native build so `<Stack stage="shared"/>` authors anywhere, and
		// this launch's deploy mechanics beside it. `Deployment` is a resource
		// rather than derived per read, so one launch publishes every artifact
		// under one id.
		app.register_type::<crate::prelude::Stack>()
			.init_resource::<crate::prelude::Deployment>();

		// the deploy `Variable` + its value resolution, a field of the blocks'
		// `env_vars` (always compiled, in `types/`).
		app.register_type::<crate::types::Variable>()
			.register_type::<crate::types::VariableValue>();

		// the two blocks a beet *application* declares (the bucket it is served
		// from, the table it records to), so `<S3BucketBlock label="app"/>` and
		// `<DynamoTableBlock label="analytics"/>` spawn by tag in any build
		// carrying their default-on binding features.
		#[cfg(feature = "bindings_aws_common")]
		app.register_type::<crate::prelude::S3BucketBlock>();
		#[cfg(feature = "bindings_aws_dynamo")]
		app.register_type::<crate::prelude::DynamoTableBlock>();

		// ..and the runtime half of those declarations: one observer per block
		// type attaching the live store, so the deploy meaning (the always
		// compiled `ErasedBlock` hook) and the runtime meaning hang off the one
		// entity the markup declared.
		#[cfg(all(
			feature = "bindings_aws_common",
			feature = "aws_sdk",
			not(target_arch = "wasm32")
		))]
		app.add_observer(crate::blocks::attach_s3_store);
		#[cfg(all(
			feature = "bindings_aws_dynamo",
			not(target_arch = "wasm32")
		))]
		app.add_observer(crate::blocks::attach_table_store);

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

		// the tofu apply action + its layer settings (`<TofuApply layer="storage"/>`),
		// and the zone edge setup/purge (the whole `actions` module is gated on
		// `deploy`, and is native-only).
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::TofuApplyAction>()
			.register_type::<crate::prelude::TofuApply>()
			.register_type::<crate::prelude::CloudflareZoneSetup>()
			.register_type::<crate::prelude::CloudflarePurgeCache>();

		// the bucket sync settings (`{SyncS3Bucket{delete:true}}`), the direction
		// enum a markup attribute names by variant, and the `<DirSync>` front-end
		// that binds a local dir to a bucket by label.
		#[cfg(all(
			feature = "deploy",
			feature = "aws_sdk",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::SyncS3Bucket>()
			.register_type::<beet_net::prelude::SyncDirection>()
			.register_type::<crate::prelude::DirSync>()
			.add_observer(crate::actions::attach_dir_sync_store);

		// the borrowed-paths copy (`<DirCopy src=".." dest=".." paths=".."/>`),
		// plain fs work so it needs no cloud backend.
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::DirCopy>();

		// the CloudWatch tail and the target it composes its log group from.
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::AwsWatch>()
			.register_type::<crate::prelude::WatchTarget>();

		// the full-lifecycle smoke-test action: reads a bucket's `BlobStore` (so
		// `aws_sdk`-gated like the store) and lives in the `actions` module (so
		// `deploy`-gated and native-only). Register it only when all three compile it.
		#[cfg(all(
			feature = "deploy",
			feature = "aws_sdk",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::LifecycleProbe>();

		// the post-apply release step: rolls a running Lightsail box onto the
		// deploy's binary, the counterpart to its machine-config-only user data.
		#[cfg(all(
			feature = "deploy",
			feature = "lightsail_block",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::LightsailRelease>()
			.register_type::<crate::prelude::LightsailReleaseAction>();

		// the docker/podman image build action + its engine selector (the
		// `build_docker_image` module is gated on `fargate_block`).
		#[cfg(all(feature = "fargate_block", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::BuildDockerImage>()
			.register_type::<crate::prelude::ContainerEngine>();
	}
}
