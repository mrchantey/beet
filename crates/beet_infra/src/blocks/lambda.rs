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
#[derive(Debug, Clone, Serialize, Deserialize, Component)]
#[require(ErasedBlock=ErasedBlock::new::<Self>())]
pub struct LambdaBlock {
	/// Optional DNS provider configuration.
	pub dns: Option<DnsProvider>,
	/// AWS region for the buckets and lambda function.
	pub region: Option<SmolStr>,
}

impl Default for LambdaBlock {
	fn default() -> Self {
		Self {
			dns: None,
			region: None,
		}
	}
}

impl Block for LambdaBlock {
	fn apply_to_config(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		let region = self.region.as_ref().unwrap_or_else(|| stack.aws_region());

		// IAM Role for Lambda
		let lambda_role = ResourceDef::new_primary(
			stack.resource_ident("lambda_role"),
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
			stack.resource_ident("lambda_basic_policy"),
			AwsIamRolePolicyAttachmentDetails {
				policy_arn: "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
					.into(),
				role: lambda_role.field_ref("name").into(),
				..default()
			},
		);

		// Lambda Function
		let lambda_function = ResourceDef::new_primary(
			stack.resource_ident("router"),
			AwsLambdaFunctionDetails {
				runtime: Some("provided.al2023".into()),
				handler: Some("bootstrap".into()),
				filename: Some("lambda.zip".into()),
				role: lambda_role.field_ref("arn").into(),
				timeout: Some(180),
				memory_size: Some(1024),
				source_code_hash: Some(
					"${filebase64sha256(\"lambda.zip\")}".into(),
				),
				..default()
			},
		);

		// Lambda Function URL
		let lambda_url = ResourceDef::new_secondary(
			stack.resource_ident("router_url"),
			AwsLambdaFunctionUrlDetails {
				authorization_type: "NONE".into(),
				function_name: lambda_function
					.field_ref("function_name")
					.into(),
				..default()
			},
		);

		// API Gateway v2
		let gateway = ResourceDef::new_primary(
			stack.resource_ident("gateway"),
			AwsApigatewayv2ApiDetails {
				protocol_type: "HTTP".into(),
				..default()
			},
		);

		let lambda_integration = ResourceDef::new_secondary(
			stack.resource_ident("lambda_integration"),
			AwsApigatewayv2IntegrationDetails {
				api_id: gateway.field_ref("id").into(),
				integration_type: "AWS_PROXY".into(),
				integration_uri: Some(
					lambda_function.field_ref("invoke_arn").into(),
				),
				payload_format_version: Some("2.0".into()),
				..default()
			},
		);

		let default_route = ResourceDef::new_secondary(
			stack.resource_ident("default_route"),
			AwsApigatewayv2RouteDetails {
				api_id: gateway.field_ref("id").into(),
				route_key: "$default".into(),
				target: Some(
					format!(
						"integrations/{}",
						lambda_integration.field_ref("id")
					)
					.into(),
				),
				..default()
			},
		);

		let default_stage = ResourceDef::new_secondary(
			stack.resource_ident("default_stage"),
			AwsApigatewayv2StageDetails {
				api_id: gateway.field_ref("id").into(),
				name: "$default".into(),
				auto_deploy: Some(true),
				..default()
			},
		);

		// Lambda Permission for API Gateway
		let apigw_permission = ResourceDef::new_secondary(
			stack.resource_ident("apigw_lambda"),
			AwsLambdaPermissionDetails {
				action: "lambda:InvokeFunction".into(),
				function_name: lambda_function
					.field_ref("function_name")
					.into(),
				principal: "apigateway.amazonaws.com".into(),
				source_arn: Some(
					format!("{}/*/*", gateway.field_ref("execution_arn"))
						.into(),
				),
				..default()
			},
		);

		// Core resources
		config
			.add_resource(&lambda_role)?
			.add_resource(&lambda_policy)?
			.add_resource(&lambda_function)?
			.add_resource(&lambda_url)?
			.add_resource(&gateway)?
			.add_resource(&lambda_integration)?
			.add_resource(&default_route)?
			.add_resource(&default_stage)?
			.add_resource(&apigw_permission)?;

		// DNS (conditional)
		if let Some(dns) = &self.dns {
			match dns {
				DnsProvider::Cloudflare { authority } => {
					let dns_def = ResourceDef::new_secondary(
						stack.resource_ident("dns"),
						CloudflareDnsRecordDetails {
							name: authority.clone(),
							ttl: 1,
							r#type: "CNAME".into(),
							zone_id: "CLOUDFLARE_ZONE_ID".into(),
							content: Some(
								gateway.field_ref("api_endpoint").into(),
							),
							proxied: Some(true),
							..default()
						},
					);
					config.add_resource(&dns_def)?;
				}
				DnsProvider::Route53 { authority, zone_id } => {
					let dns_def = ResourceDef::new_secondary(
						stack.resource_ident("dns"),
						AwsRoute53RecordDetails {
							name: authority.clone(),
							r#type: "CNAME".into(),
							zone_id: zone_id.clone(),
							ttl: Some(300),
							records: Some(vec![
								gateway.field_ref("api_endpoint").into(),
							]),
							..default()
						},
					);
					config.add_resource(&dns_def)?;
				}
			}
		}

		// Outputs
		config
			.add_output("api_endpoint", terra::Output {
				value: json!(gateway.field_ref("api_endpoint")),
				description: Some("The API Gateway endpoint URL".into()),
				sensitive: None,
			})?
			.add_output("function_url", terra::Output {
				value: json!(lambda_url.field_ref("function_url")),
				description: Some("The Lambda function URL".into()),
				sensitive: None,
			})?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test(timeout_ms = 120000)]
	#[ignore = "very slow"]
	async fn validate() {
		let (stack, _dir) = Stack::default_local();
		let block = LambdaBlock::default();
		let mut config = stack.create_config();
		block.apply_to_config(&stack, &mut config).unwrap();
		let project = terra::Project::new(&stack, config);
		project.validate().await.unwrap();
	}
}
