#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
//! Integration test for Lightsail Block.
//! Takes approx 10 mins.
use beet_core::prelude::*;
use beet_infra::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

mod runtime_utils;
use runtime_utils::*;

const EXAMPLE_NAME: &str = "lightsail_test";
const SOURCE_PATH: &str = "crates/beet_infra/examples/lightsail_test.rs";

#[beet_core::test(timeout_ms = 900_000)]
#[ignore = "deploys resources and takes ten minutes"]
async fn lightsail_lifecycle() {
	pretty_env_logger::init();

	// resolve source paths and create isolated temp assets to prevent
	// interference with concurrent tests
	let guards = setup_isolated_test_guards(SOURCE_PATH).unwrap();
	let source = &guards.source;
	let assets_file = &guards.assets_file;
	let assets_dir = &guards.assets_dir;

	let mut stack = Stack::new("lightsail-test").with_aws_region("us-west-2");

	// clean up any prior state
	let project = build_project(&stack).unwrap();
	cleanup_prior_state(&stack, project).await;

	// 1. deploy v1
	info!("step 1: deploying v1");
	deploy(&stack, assets_dir).await.unwrap();

	// 2. verify v1 is live
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 2: verifying v1 at {address}");
	verify_live(&address, MARKER_V1).await.unwrap();

	// 3. verify v1 assets
	info!("step 3: verifying v1 assets");
	verify_assets(&stack, MARKER_V1).await.unwrap();

	// 4-5. modify source and assets to v2, deploy again
	info!("step 4-5: deploying v2");
	swap_version(source, MARKER_V1, MARKER_V2).unwrap();
	swap_version(assets_file, MARKER_V1, MARKER_V2).unwrap();
	stack = Stack::new("lightsail-test").with_aws_region("us-west-2");
	deploy(&stack, assets_dir).await.unwrap();

	// 6. verify v2
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 6: verifying v2 at {address}");
	verify_live(&address, MARKER_V2).await.unwrap();

	// 7. verify v2 assets
	info!("step 7: verifying v2 assets");
	verify_assets(&stack, MARKER_V2).await.unwrap();

	// 8. rollback to v1
	info!("step 8: rolling back");
	let client = stack.artifacts_client();
	client.rollback(1).await.unwrap();
	apply_with_current_ledger(&mut stack, build_project)
		.await
		.unwrap();

	// 9. verify v1 after rollback
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 9: verifying v1 after rollback at {address}");
	verify_live(&address, MARKER_V1).await.unwrap();

	// 10. verify v1 assets after rollback
	info!("step 10: verifying v1 assets after rollback");
	verify_assets(&stack, MARKER_V1).await.unwrap();

	// 11. rollforward to v2
	info!("step 11: rolling forward");
	let client = stack.artifacts_client();
	client.rollforward().await.unwrap();
	apply_with_current_ledger(&mut stack, build_project)
		.await
		.unwrap();

	// 12. verify v2 after rollforward
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 12: verifying v2 after rollforward at {address}");
	verify_live(&address, MARKER_V2).await.unwrap();

	// 13. verify v2 assets after rollforward
	info!("step 13: verifying v2 assets after rollforward");
	verify_assets(&stack, MARKER_V2).await.unwrap();

	// 14. destroy
	info!("step 14: destroying");
	build_project(&stack).unwrap().destroy().await.unwrap();

	// 15. verify dead
	info!("step 15: verifying dead");
	verify_dead(&address, 10, 5, 5).await.unwrap();

	// revert source file (guard handles assets)
	swap_version(source, MARKER_V2, MARKER_V1).ok();
}

/// Build the terraform project for the Lightsail test stack.
fn build_project(stack: &Stack) -> Result<terra::Project> {
	let block = LightsailBlock::default();
	let bucket_block = assets_bucket_block();
	let mut config = stack.create_config();
	config.add_provider_config(
		&terra::Provider::AWS,
		&serde_json::json!({ "region": stack.aws_region() }),
	)?;
	let mut world = World::new();
	let entity_mut = world.spawn(());
	let entity = entity_mut.as_readonly();
	block.apply_to_config(&entity, stack, &mut config)?;
	bucket_block.apply_to_config(&entity, stack, &mut config)?;
	terra::Project::new(stack, config).xok()
}

/// Build, upload artifacts, sync assets, and apply terraform
/// using the deploy action sequence.
async fn deploy(stack: &Stack, assets_dir: &AbsPathBuf) -> Result {
	let block = LightsailBlock::default();
	let cargo = CargoBuild::default()
		.with_release(true)
		.with_target(BuildTarget::Zigbuild)
		.with_package("beet_infra")
		.with_example(EXAMPLE_NAME)
		.with_additional_args(vec!["--features".into(), "deploy".into()])
		.into_build_artifact();

	let _response = AsyncPlugin::world()
		.spawn((
			stack.clone(),
			assets_s3_fs_bucket(stack, assets_dir),
			assets_bucket_block(),
			exchange_sequence(),
			children![(block, cargo), TofuApplyAction, SyncS3BucketAction,],
		))
		.exchange(Request::get(""))
		.await
		.into_result()
		.await?;
	info!("deploy complete");
	Ok(())
}

/// Lightsail-specific verify_live wrapper.
/// Lightsail instances serve on port 80 via public IP.
async fn verify_live(address: &str, expected: &str) -> Result {
	let url = format!("http://{address}/version");
	runtime_utils::verify_live(&url, expected, 30, 10).await
}
