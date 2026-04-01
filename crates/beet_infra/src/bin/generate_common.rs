//! Generate commonly used types.
//! Codegen relies on open-tofu installation and takes tens of seconds.
//! Many applications require only a handful of common infra resources,
//! so we commit these types to the repository.
//!
//! Run with:
//!     cargo run -p beet_infra --bin generate_common --features bindings_generator

use beet_core::prelude::*;
use beet_infra::bindings_generator::schema_binding_generator::BindingFile;
use beet_infra::bindings_generator::schema_binding_generator::SchemaBindingGenerator;
use beet_infra::config_exporter::types::TerraProvider;

#[beet_core::main]
async fn main() -> Result {
	let generator = SchemaBindingGenerator::default()
		// AWS resources used by examples/infra/lambda.rs
		.with_file(
			BindingFile::new(
				"crates/beet_infra/src/common_resources/aws_lambda.rs",
			)
			.with_resources(TerraProvider::AWS, [
				"aws_api_gateway_rest_api",
				"aws_apigatewayv2_api",
				"aws_apigatewayv2_integration",
				"aws_apigatewayv2_route",
				"aws_apigatewayv2_stage",
				"aws_iam_role",
				"aws_iam_role_policy_attachment",
				"aws_lambda_function",
				"aws_lambda_function_url",
				"aws_lambda_permission",
				"aws_s3_bucket",
			]),
		)
		// AWS resources used by examples/infra/lightsail.rs
		.with_file(
			BindingFile::new(
				"crates/beet_infra/src/common_resources/aws_lightsail.rs",
			)
			.with_resources(TerraProvider::AWS, [
				"aws_lightsail_instance",
				"aws_lightsail_instance_public_ports",
				"aws_lightsail_key_pair",
				"aws_lightsail_static_ip",
				"aws_lightsail_static_ip_attachment",
			]),
		)
		// Cloudflare resources used by examples/infra/lambda.rs
		.with_file(
			BindingFile::new(
				"crates/beet_infra/src/common_resources/cloudflare_dns.rs",
			)
			.with_resources(TerraProvider::CLOUDFLARE, [
				"cloudflare_dns_record",
			]),
		);

	// Use the existing schema.json instead of running the full tofu init cycle.
	beet_core::cross_log!("Generating provider bindings from schema.json ...");
	generator.generate().await?;
	beet_core::cross_log!(
		"Done! Provider modules written to src/common_resources/"
	);

	Ok(())
}
