//! Generate commonly used types.
//! Codegen relies on open-tofu installation and takes tens of seconds.
//! Many applications require only a handful of common infra resources,
//! so we commit these types to the repository.
//!
//! Reproducible: each provider is pinned to its exact
//! [`schema_version`](terra::Provider::schema_version), so the same command
//! yields the same tree on every machine. To move to a newer provider, bump
//! the pin and rerun.
//!
//! Run with `just bindings`, or:
//!     cargo run -p beet_infra --bin bindings --features bindings_generator
//!
//! `just bindings-verify` regenerates and fails on a dirty tree.
//!
//! Native-only, like the generator it drives. A wasm build of beet_infra still
//! compiles this bin target, so there it is a stub.

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
use beet_core::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use beet_infra::bindings_generator::BindingFile;
#[cfg(not(target_arch = "wasm32"))]
use beet_infra::bindings_generator::SchemaBindingGenerator;
#[cfg(not(target_arch = "wasm32"))]
use beet_infra::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
#[beet_core::main]
async fn main() -> Result {
	// Use the existing schema.json instead of running the full tofu init cycle.
	info!("Generating provider bindings from schema.json ...");

	SchemaBindingGenerator::default()
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_common.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_cloudwatch_log_group",
					"aws_cloudwatch_metric_alarm",
					"aws_iam_access_key",
					"aws_iam_role",
					"aws_iam_role_policy",
					"aws_iam_role_policy_attachment",
					"aws_iam_user",
					"aws_iam_user_policy",
					"aws_iam_user_policy_attachment",
					"aws_s3_bucket",
					"aws_s3_bucket_policy",
					"aws_s3_bucket_public_access_block",
				]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_dynamo.rs")
				.with_resources(terra::Provider::AWS, ["aws_dynamodb_table"]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_dns.rs")
				.with_resources(terra::Provider::AWS, ["aws_route53_record"]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_acm.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_acm_certificate",
					"aws_acm_certificate_validation",
				]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_lambda.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_api_gateway_rest_api",
					"aws_apigatewayv2_api",
					"aws_apigatewayv2_api_mapping",
					"aws_apigatewayv2_domain_name",
					"aws_apigatewayv2_integration",
					"aws_apigatewayv2_route",
					"aws_apigatewayv2_stage",
					"aws_lambda_function",
					"aws_lambda_function_url",
					"aws_lambda_permission",
				]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_lightsail.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_lightsail_instance",
					"aws_lightsail_instance_public_ports",
					"aws_lightsail_key_pair",
					"aws_lightsail_static_ip",
					"aws_lightsail_static_ip_attachment",
				]),
		)
		.with_file(
			BindingFile::new(
				"crates/beet_infra/src/bindings/aws_autoscaling.rs",
			)
			.with_resources(terra::Provider::AWS, [
				"aws_appautoscaling_policy",
				"aws_appautoscaling_target",
			]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_fargate.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_ecr_repository",
					"aws_ecs_cluster",
					"aws_ecs_service",
					"aws_ecs_task_definition",
					"aws_lb",
					"aws_lb_listener",
					"aws_lb_target_group",
				]),
		)
		// the network every AWS compute block sits in, its own file because a
		// vpc is not a fargate thing: the mail stack's box and database need the
		// same resources with no ECS in sight.
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_vpc.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_internet_gateway",
					"aws_route",
					"aws_route_table",
					"aws_route_table_association",
					"aws_security_group",
					"aws_security_group_rule",
					"aws_subnet",
					"aws_vpc",
				]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_ec2.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_instance",
					"aws_eip",
					"aws_eip_association",
					"aws_key_pair",
					"aws_iam_instance_profile",
				]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_rds.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_db_instance",
					"aws_db_subnet_group",
				]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_ses.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_sesv2_email_identity",
					"aws_sesv2_email_identity_mail_from_attributes",
					"aws_sesv2_configuration_set",
					"aws_sesv2_configuration_set_event_destination",
				]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_ssm.rs")
				.with_resources(terra::Provider::AWS, ["aws_ssm_parameter"]),
		)
		// EventBridge Scheduler, the timer a `ScheduledJobBlock` declares. The
		// invoke role it needs is an `aws_iam_role` + `aws_iam_role_policy`
		// from `aws_common`, so nothing else belongs here.
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_scheduler.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_scheduler_schedule",
				]),
		)
		.with_file(
			BindingFile::new(
				"crates/beet_infra/src/bindings/cloudflare_common.rs",
			)
			.with_resources(terra::Provider::CLOUDFLARE, [
				"cloudflare_dns_record",
				"cloudflare_load_balancer",
				"cloudflare_load_balancer_pool",
				"cloudflare_load_balancer_monitor",
			]),
		)
		// NOTE the cloudflare zone-level edge config (cache ruleset, zone settings)
		// is deliberately NOT terraform: entrypoint rulesets are zone singletons
		// that fight stack-scoped state, so the `CloudflareZoneSetup` action
		// manages them through the idempotent zone APIs instead (the plan-polymorphic
		// spectrum API was also incompatible: the provider always sends
		// Enterprise-only fields, which non-Enterprise zones reject).
		.generate()
		.await?;
	info!("Done!");
	Ok(())
}
