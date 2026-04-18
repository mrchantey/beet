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
#[derive(Debug, Clone, Get, SetWith, Serialize, Deserialize, Component)]
#[component(immutable, on_add = ErasedBlock::on_add::<LambdaBlock>)]
pub struct LambdaBlock {
	/// Label used as a prefix for all terraform resources,
	/// variables, and outputs. Also used as the artifact name.
	/// Defaults to `main-lambda`
	label: SmolStr,
	/// Optional DNS provider configuration.
	pub dns: Option<DnsProvider>,
	/// AWS region for the buckets and lambda function.
	pub region: Option<SmolStr>,
}


impl Default for LambdaBlock {
	fn default() -> Self {
		Self {
			label: "main-lambda".into(),
			dns: None,
			region: None,
		}
	}
}

impl LambdaBlock {
	/// Build a prefixed label for terraform resources, variables, and outputs.
	pub fn build_label(&self, suffix: &str) -> String {
		format!("{}--{}", self.label, suffix)
	}
}

impl Block for LambdaBlock {
	fn artifact_label(&self) -> Option<&str> { Some(&self.label) }
	fn apply_to_config(
		&self,
		entity: &EntityRef,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		let region = self.region.as_ref().unwrap_or_else(|| stack.aws_region());

		// artifact values computed directly from stack and entity
		let artifact_bucket = stack.artifact_bucket_name();
		let artifact_key = stack.artifact_key(&self.label);
		let source_hash = entity
			.get::<BuildArtifact>()
			.and_then(|artifact| artifact.compute_source_hash().ok());

		// IAM Role for Lambda
		let lambda_role = ResourceDef::new_primary(
			stack.resource_ident(self.build_label("lambda_role")),
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
			stack.resource_ident(self.build_label("lambda_basic_policy")),
			AwsIamRolePolicyAttachmentDetails {
				policy_arn: "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
					.into(),
				role: lambda_role.field_ref("name").into(),
				..default()
			},
		);

		// S3 Read Access for Lambda to read assets and artifacts
		let s3_read_policy = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("s3_read_policy")),
			AwsIamRolePolicyAttachmentDetails {
				policy_arn: "arn:aws:iam::aws:policy/AmazonS3ReadOnlyAccess"
					.into(),
				role: lambda_role.field_ref("name").into(),
				..default()
			},
		);

		// Lambda Function
		let lambda_function = ResourceDef::new_primary(
			stack.resource_ident(self.build_label("function")),
			AwsLambdaFunctionDetails {
				runtime: Some("provided.al2023".into()),
				handler: Some("bootstrap".into()),
				filename: None,
				s3_bucket: Some(artifact_bucket.into()),
				s3_key: Some(artifact_key.into()),
				region: Some(region.clone()),
				role: lambda_role.field_ref("arn").into(),
				timeout: Some(180),
				memory_size: Some(1024),
				source_code_hash: source_hash.map(Into::into),
				..default()
			},
		);

		// Lambda Function URL
		let lambda_url = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("function_url")),
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
			stack.resource_ident(self.build_label("gateway")),
			AwsApigatewayv2ApiDetails {
				protocol_type: "HTTP".into(),
				..default()
			},
		);

		let lambda_integration = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("lambda_integration")),
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
			stack.resource_ident(self.build_label("default_route")),
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
			stack.resource_ident(self.build_label("default_stage")),
			AwsApigatewayv2StageDetails {
				api_id: gateway.field_ref("id").into(),
				name: "$default".into(),
				auto_deploy: Some(true),
				..default()
			},
		);

		// Lambda Permission for API Gateway
		let apigw_permission = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("apigw_lambda")),
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
			.add_resource(&s3_read_policy)?
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
						stack.resource_ident(self.build_label("dns")),
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
						stack.resource_ident(self.build_label("dns")),
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
			.add_output(self.build_label("api_endpoint"), terra::Output {
				value: json!(gateway.field_ref("api_endpoint")),
				description: Some("The API Gateway endpoint URL".into()),
				sensitive: None,
			})?
			.add_output(self.build_label("function_url"), terra::Output {
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
		let mut world = World::new();
		block
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&mut config,
			)
			.unwrap();
		let project = terra::Project::new(&stack, config);
		project.validate().await.unwrap();
	}
}
