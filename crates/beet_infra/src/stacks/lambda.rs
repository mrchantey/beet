use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DnsProvider {
	Cloudflare {
		authority: SmolStr,
	},
	Route53 {
		authority: SmolStr,
		zone_id: SmolStr,
	},
}

/// Opinionated terraform configuration for a standard web application:
/// - Serverless lambda function with API Gateway v2
/// - HTML and assets S3 buckets
/// - Optional DNS configuration (Cloudflare or Route53)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaStack {
	/// Optional DNS provider configuration.
	pub dns: Option<DnsProvider>,
	/// AWS region for the buckets and lambda function.
	pub region: Option<SmolStr>,
}

impl Default for LambdaStack {
	fn default() -> Self {
		Self {
			dns: None,
			region: None,
		}
	}
}

impl LambdaStack {
	fn region(&self) -> &str {
		self.region.as_deref().unwrap_or(aws::region::DEFAULT)
	}

	/// Build a complete [`terra::Config`] for this stack.
	pub fn build_config(
		&self,
		cx: &StackContext,
		stack: &Stack,
	) -> terra::Config {
		let region = self.region();

		// S3 Buckets
		let html_bucket = ResourceDef::new_primary(
			cx.resource_ident("html"),
			AwsS3BucketDetails {
				force_destroy: Some(true),
				region: Some(region.into()),
				..default()
			},
		);
		let assets_bucket = ResourceDef::new_primary(
			cx.resource_ident("assets"),
			AwsS3BucketDetails {
				force_destroy: Some(true),
				region: Some(region.into()),
				..default()
			},
		);

		// IAM Role for Lambda
		let lambda_role = ResourceDef::new_primary(
			cx.resource_ident("lambda_role"),
			AwsIamRoleDetails {
				assume_role_policy: json!({
					"Version": "2012-10-17",
					"Statement": [{
						"Action": "sts:AssumeRole",
						"Effect": "Allow",
						"Principal": { "Service": "lambda.amazonaws.com" }
					}]
				})
				.to_string()
				.into(),
				..default()
			},
		);

		// IAM Role Policy Attachment
		let lambda_policy = ResourceDef::new_secondary(
			cx.resource_ident("lambda_basic_policy"),
			AwsIamRolePolicyAttachmentDetails {
				policy_arn: "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole".into(),
				role: terra::tf_ref(&lambda_role.field("name")),
				..default()
			},
		);

		// Lambda Function
		let lambda_function = ResourceDef::new_primary(
			cx.resource_ident("router"),
			AwsLambdaFunctionDetails {
				runtime: Some("provided.al2023".into()),
				handler: Some("bootstrap".into()),
				filename: Some("lambda.zip".into()),
				role: terra::tf_ref(&lambda_role.field("arn")),
				timeout: Some(180),
				memory_size: Some(1024),
				source_code_hash: Some(default()),
				..default()
			},
		);

		// Lambda Function URL
		let lambda_url = ResourceDef::new_secondary(
			cx.resource_ident("router_url"),
			AwsLambdaFunctionUrlDetails::new(
				"NONE".into(),
				terra::tf_ref(&lambda_function.field("function_name")),
			),
		);

		// API Gateway v2
		let gateway_ident = cx.resource_ident("gateway");
		let gateway = ResourceDef::new_secondary(
			gateway_ident.clone(),
			AwsApigatewayv2ApiDetails::new(
				gateway_ident.primary_identifier().into(),
				"HTTP".into(),
			),
		);

		let mut lambda_integration_details =
			AwsApigatewayv2IntegrationDetails::new(
				terra::tf_ref(&gateway.field("id")),
				"AWS_PROXY".into(),
			);
		lambda_integration_details.integration_uri =
			Some(terra::tf_ref(&lambda_function.field("invoke_arn")));
		lambda_integration_details.payload_format_version = Some("2.0".into());

		let lambda_integration = ResourceDef::new_secondary(
			cx.resource_ident("lambda_integration"),
			lambda_integration_details,
		);

		let mut default_route_details = AwsApigatewayv2RouteDetails::new(
			terra::tf_ref(&gateway.field("id")),
			"$default".into(),
		);
		default_route_details.target = Some(
			format!("integrations/${{{}}}", lambda_integration.field("id"))
				.into(),
		);
		let default_route = ResourceDef::new_secondary(
			cx.resource_ident("default_route"),
			default_route_details,
		);

		let mut default_stage_details = AwsApigatewayv2StageDetails::new(
			terra::tf_ref(&gateway.field("id")),
			"$default".into(),
		);
		default_stage_details.auto_deploy = Some(true);
		let default_stage = ResourceDef::new_secondary(
			cx.resource_ident("default_stage"),
			default_stage_details,
		);

		// Lambda Permission for API Gateway
		let mut apigw_permission_details = AwsLambdaPermissionDetails::new(
			"lambda:InvokeFunction".into(),
			terra::tf_ref(&lambda_function.field("function_name")),
			"apigateway.amazonaws.com".into(),
		);
		apigw_permission_details.source_arn =
			Some(format!("${{{}}}/*/*", gateway.field("execution_arn")).into());
		let apigw_permission = ResourceDef::new_secondary(
			cx.resource_ident("apigw_lambda"),
			apigw_permission_details,
		);

		// Assemble core resources
		let mut config = terra::Config::default()
			.with_backend(stack.backend())
			.with_resource(&html_bucket)
			.with_resource(&assets_bucket)
			.with_resource(&lambda_role)
			.with_resource(&lambda_policy)
			.with_resource(&lambda_function)
			.with_resource(&lambda_url)
			.with_resource(&gateway)
			.with_resource(&lambda_integration)
			.with_resource(&default_route)
			.with_resource(&default_stage)
			.with_resource(&apigw_permission);

		// DNS (conditional)
		if let Some(dns) = &self.dns {
			match dns {
				DnsProvider::Cloudflare { authority } => {
					let mut dns_record = CloudflareDnsRecordDetails::new(
						authority.clone(),
						1,
						"CNAME".into(),
						"CLOUDFLARE_ZONE_ID".into(),
					);
					dns_record.content =
						Some(terra::tf_ref(&gateway.field("api_endpoint")));
					dns_record.proxied = Some(true);
					let dns_def = ResourceDef::new_secondary(
						cx.resource_ident("dns"),
						dns_record,
					);
					config = config.with_resource(&dns_def);
				}
				DnsProvider::Route53 { authority, zone_id } => {
					let mut dns_record = AwsRoute53RecordDetails::new(
						authority.clone(),
						"CNAME".into(),
						zone_id.clone(),
					);
					dns_record.ttl = Some(300);
					dns_record.records = Some(vec![terra::tf_ref(
						&gateway.field("api_endpoint"),
					)]);
					let dns_def = ResourceDef::new_secondary(
						cx.resource_ident("dns"),
						dns_record,
					);
					config = config.with_resource(&dns_def);
				}
			}
		}

		// Outputs
		config
			.with_output("api_endpoint", terra::Output {
				value: json!(
					terra::tf_ref(&gateway.field("api_endpoint")).as_str()
				),
				description: Some("The API Gateway endpoint URL".into()),
				sensitive: None,
			})
			.with_output("function_url", terra::Output {
				value: json!(
					terra::tf_ref(&lambda_url.field("function_url")).as_str()
				),
				description: Some("The Lambda function URL".into()),
				sensitive: None,
			})
			.with_output("assets_bucket", terra::Output {
				value: json!(
					terra::tf_ref(&assets_bucket.field("bucket")).as_str()
				),
				description: Some("The S3 assets bucket name".into()),
				sensitive: None,
			})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test(timeout_ms = 120000)]
	async fn lambda_config_validates() {
		let cx = StackContext::default();
		let stack = Stack::new(LocalBackend::default());
		let lambda = LambdaStack::default();
		let config = lambda.build_config(&cx, &stack);
		config.validate().await.unwrap();
	}
}
