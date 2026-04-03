//! Generate commonly used types.
//! Codegen relies on open-tofu installation and takes tens of seconds.
//! Many applications require only a handful of common infra resources,
//! so we commit these types to the repository.
//!
//! Run with:
//!     cargo run -p beet_infra --bin generate_common --features bindings_generator

use beet_core::prelude::*;
use beet_infra::bindings_generator::BindingFile;
use beet_infra::bindings_generator::SchemaBindingGenerator;
use beet_infra::prelude::*;

#[beet_core::main]
async fn main() -> Result {
	let generator = SchemaBindingGenerator::default()
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_common.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_iam_role",
					"aws_iam_role_policy_attachment",
					"aws_s3_bucket",
				]),
		)
		.with_file(
			BindingFile::new("crates/beet_infra/src/bindings/aws_lambda.rs")
				.with_resources(terra::Provider::AWS, [
					"aws_api_gateway_rest_api",
					"aws_apigatewayv2_api",
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
				"crates/beet_infra/src/bindings/cloudflare_common.rs",
			)
			.with_resources(terra::Provider::CLOUDFLARE, [
				"cloudflare_dns_record",
			]),
		);

	// Use the existing schema.json instead of running the full tofu init cycle.
	beet_core::cross_log!("Generating provider bindings from schema.json ...");
	generator.generate().await?;
	beet_core::cross_log!("Done!");

	Ok(())
}
