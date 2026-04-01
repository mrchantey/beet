//! Lambda + API Gateway + Cloudflare DNS example using the typed provider API.
//!
//! Run with:
//! ```sh
//!   cargo run --example lambda --features=fs,rand,infra_aws_lambda,infra_cloudflare_dns
//! ```

use beet::prelude::*;
use beet_infra::common_resources::aws_lambda::*;
use beet_infra::common_resources::cloudflare_dns::*;
use beet_infra::config_exporter::config_exporter::ConfigExporter;
use beet_infra::config_exporter::config_exporter::Output;
use serde_json::json;

#[beet::main]
async fn main() -> Result {
	let app_name = "beet-site";
	let stage = "dev";
	let prefix = format!("{app_name}--{stage}");

	// -----------------------------------------------------------------
	// S3 Buckets
	// -----------------------------------------------------------------

	let assets_bucket = AwsS3BucketDetails {
		bucket: Some(format!("{prefix}--assets")),
		force_destroy: Some(true),
		..Default::default()
	};

	let html_bucket = AwsS3BucketDetails {
		bucket: Some(format!("{prefix}--html")),
		force_destroy: Some(true),
		..Default::default()
	};

	// -----------------------------------------------------------------
	// IAM Role for Lambda
	// -----------------------------------------------------------------

	let assume_role_policy = json!({
		"Version": "2012-10-17",
		"Statement": [{
			"Action": "sts:AssumeRole",
			"Effect": "Allow",
			"Principal": { "Service": "lambda.amazonaws.com" }
		}]
	});

	let mut lambda_role =
		AwsIamRoleDetails::new(assume_role_policy.to_string());
	lambda_role.name = Some(format!("{prefix}--lambda-role"));

	let lambda_basic_policy = AwsIamRolePolicyAttachmentDetails::new(
		"arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
			.into(),
		"${aws_iam_role.lambda_role.name}".into(),
	);

	// -----------------------------------------------------------------
	// Lambda Function + URL
	// -----------------------------------------------------------------

	let mut router = AwsLambdaFunctionDetails::new(
		format!("{prefix}--router"),
		"${aws_iam_role.lambda_role.arn}".into(),
	);
	router.runtime = Some("provided.al2023".into());
	router.handler = Some("bootstrap".into());
	router.filename = Some("lambda.zip".into());
	router.timeout = Some(180);
	router.memory_size = Some(1024);
	router.source_code_hash = Some(String::new());

	let router_url = AwsLambdaFunctionUrlDetails::new(
		"NONE".into(),
		"${aws_lambda_function.router.function_name}".into(),
	);

	// -----------------------------------------------------------------
	// API Gateway v2
	// -----------------------------------------------------------------

	let gateway = AwsApigatewayv2ApiDetails::new(
		format!("{prefix}--gateway"),
		"HTTP".into(),
	);

	let mut lambda_integration = AwsApigatewayv2IntegrationDetails::new(
		"${aws_apigatewayv2_api.gateway.id}".into(),
		"AWS_PROXY".into(),
	);
	lambda_integration.integration_uri =
		Some("${aws_lambda_function.router.invoke_arn}".into());
	lambda_integration.payload_format_version = Some("2.0".into());

	let mut default_route = AwsApigatewayv2RouteDetails::new(
		"${aws_apigatewayv2_api.gateway.id}".into(),
		"$default".into(),
	);
	default_route.target = Some(
		"integrations/${aws_apigatewayv2_integration.lambda_integration.id}"
			.into(),
	);

	let mut default_stage = AwsApigatewayv2StageDetails::new(
		"${aws_apigatewayv2_api.gateway.id}".into(),
		"$default".into(),
	);
	default_stage.auto_deploy = Some(true);

	// -----------------------------------------------------------------
	// Lambda Permission for API Gateway
	// -----------------------------------------------------------------

	let mut apigw_permission = AwsLambdaPermissionDetails::new(
		"lambda:InvokeFunction".into(),
		"${aws_lambda_function.router.function_name}".into(),
		"apigateway.amazonaws.com".into(),
	);
	apigw_permission.source_arn =
		Some("${aws_apigatewayv2_api.gateway.execution_arn}/*/*".into());

	// -----------------------------------------------------------------
	// Cloudflare DNS — point domain at the API Gateway
	// -----------------------------------------------------------------

	let mut dns_record = CloudflareDnsRecordDetails::new(
		format!("{stage}.beetstack.dev"),
		1, // TTL=1 means "automatic" in Cloudflare
		"CNAME".into(),
		"CLOUDFLARE_ZONE_ID".into(), // replace at deploy time / use a variable
	);
	dns_record.content =
		Some("${aws_apigatewayv2_api.gateway.api_endpoint}".into());
	dns_record.proxied = Some(true);

	// -----------------------------------------------------------------
	// Assemble the config
	// -----------------------------------------------------------------

	let exporter = ConfigExporter::new()
		.with_resource("assets", &assets_bucket)
		.with_resource("html", &html_bucket)
		.with_resource("lambda_role", &lambda_role)
		.with_resource("lambda_basic", &lambda_basic_policy)
		.with_resource("router", &router)
		.with_resource("router_url", &router_url)
		.with_resource("gateway", &gateway)
		.with_resource("lambda_integration", &lambda_integration)
		.with_resource("default_route", &default_route)
		.with_resource("default_stage", &default_stage)
		.with_resource("apigw_lambda", &apigw_permission)
		.with_resource("domain", &dns_record)
		.with_output("api_endpoint", Output {
			value: json!("${aws_apigatewayv2_api.gateway.api_endpoint}"),
			description: Some("The API Gateway endpoint URL".into()),
			sensitive: None,
		})
		.with_output("function_url", Output {
			value: json!("${aws_lambda_function_url.router_url.function_url}"),
			description: Some("The Lambda function URL".into()),
			sensitive: None,
		})
		.with_output("assets_bucket", Output {
			value: json!("${aws_s3_bucket.assets.bucket}"),
			description: Some("The S3 assets bucket name".into()),
			sensitive: None,
		});

	// -----------------------------------------------------------------
	// Export + validate
	// -----------------------------------------------------------------

	let dir = TempDir::new()?;
	let out_path = dir.join("main.tf.json");

	let result = exporter.export_and_validate(&out_path).await?;
	beet::cross_log!("tofu validate: {result}");

	Ok(())
}
