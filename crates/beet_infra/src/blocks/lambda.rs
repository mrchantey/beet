use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

/// Opinionated terraform configuration for a standard web application:
/// - Serverless lambda function with API Gateway v2
/// - HTML and assets S3 buckets
/// - Custom domains with a DNS-validated ACM certificate, when [`dns`] is set
///
/// [`dns`]: Self::dns
#[derive(Debug, Clone, Get, SetWith, Serialize, Deserialize, Component)]
#[component(immutable, on_add = ErasedBlock::on_add::<LambdaBlock>)]
pub struct LambdaBlock {
	/// Label used as a prefix for all terraform resources,
	/// variables, and outputs. Also used as the artifact name.
	/// Defaults to `main-lambda`
	label: SmolStr,
	/// Tofu variables to be inserted as environment variables
	/// in the lambda function.
	#[serde(default)]
	env_vars: Vec<Variable>,
	/// The public hostnames this function serves, each an api gateway custom
	/// domain with a public record pointing at it (first is the certificate's
	/// primary domain, rest are SANs). Empty leaves only the gateway's default
	/// `execute-api` endpoint, which routes by `Host` header and so answers no
	/// other name.
	#[serde(default)]
	#[set_with(skip)]
	dns: Vec<DnsProvider>,
	/// AWS region for the buckets and lambda function.
	region: Option<SmolStr>,
}

impl Default for LambdaBlock {
	fn default() -> Self {
		Self {
			label: "main-lambda".into(),
			dns: Vec::new(),
			region: None,
			env_vars: Vec::new(),
		}
	}
}

impl LambdaBlock {
	/// Build a prefixed label for terraform resources, variables, and outputs.
	pub fn build_label(&self, suffix: &str) -> String {
		format!("{}--{}", self.label, suffix)
	}

	/// Add a hostname the function answers (the first added is the ACM cert's
	/// primary domain, the rest are subject alternative names). See
	/// [`dns`](Self::dns).
	pub fn with_dns(mut self, dns: DnsProvider) -> Self {
		self.dns.push(dns);
		self
	}
}

impl Block for LambdaBlock {
	fn artifact_label(&self) -> Option<&str> { Some(&self.label) }
	fn variables(&self) -> Vec<Variable> { self.env_vars.clone() }
	fn apply_to_config(
		&self,
		entity: &EntityRef,
		stack: &ResolvedStack,
		deployment: &Deployment,
		_access: &AccessGrants,
		config: &mut terra::Config,
	) -> Result {
		let region = self
			.region
			.clone()
			.unwrap_or_else(|| stack.region().clone());
		// artifact values computed directly from the deploy and entity
		let artifact_bucket = deployment.artifact_bucket_name(stack);
		let artifact_key = deployment.artifact_key(&self.label);
		cfg_if! {
			// `BuildArtifact` holds a `ChildProcess`, so the hash of a
			// locally-built artifact only exists where the build can run.
			if #[cfg(all(feature = "deploy", not(target_arch = "wasm32")))] {
				let source_hash = entity
					.get::<BuildArtifact>()
					.and_then(|artifact| artifact.compute_source_hash().ok());
			} else {
				let source_hash: Option<String> = None;
			}
		}

		// CloudWatch log group for Lambda logs
		// Must be created before the Lambda function to ensure proper cleanup
		let function_ident = stack.resource_ident(self.build_label("function"));
		let log_group = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("logs")),
			AwsCloudwatchLogGroupDetails {
				name: Some(
					format!(
						"/aws/lambda/{}",
						function_ident.primary_identifier()
					)
					.into(),
				),
				retention_in_days: Some(30),
				..default()
			},
		);

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

		// declare terraform variables for env_vars
		for variable in &self.env_vars {
			config.ensure_variable(
				variable.key().as_str(),
				variable.tf_declaration(),
			);
		}

		// Lambda Function
		let lambda_function = ResourceDef::new_primary(
			function_ident,
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
				environment: Some(vec![
					AwsLambdaFunctionResourceBlockTypeEnvironment {
						variables: Some({
							let mut vars = std::collections::BTreeMap::new();
							// the deploy identity, named by the same table the
							// runtime parses.
							let runtime = BootstrapConfig {
								deploy_id: Some(
									deployment.deploy_id().to_string().into(),
								),
								deploy_timestamp: Some(
									deployment
										.deploy_timestamp()
										.to_string()
										.into(),
								),
								// the stage the function actually runs in, or it
								// reports the `dev` default from a prod stack
								stage: stack.stage().clone(),
								..default()
							};
							for (key, value) in runtime.to_env() {
								vars.insert(key, value.to_string().into());
							}
							// add env_vars as terraform variable references
							for variable in &self.env_vars {
								vars.insert(
									variable.key().clone(),
									variable.tf_var_ref().into(),
								);
							}
							vars
						}),
					},
				]),
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
			.add_resource(&log_group)?
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

		// DNS (conditional): a custom domain per authority, and the public record
		// pointing at it.
		self.emit_custom_domains(
			stack,
			config,
			&region,
			&gateway,
			&default_stage,
		)?;

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

impl LambdaBlock {
	/// Publish every [`dns`](Self::dns) authority as an api gateway custom
	/// domain: one DNS-validated ACM certificate covering the whole set, a
	/// `domain_name` + `api_mapping` per authority, and a public record pointing
	/// each authority at its domain's `target_domain_name`.
	///
	/// The custom domain is what makes a proxying CDN work at all: the gateway's
	/// default endpoint routes by `Host` header, so a record pointing a site
	/// hostname at it is answered with a 403. Terminating that hostname here also
	/// gives the edge-to-origin leg a certificate covering the name it asked for,
	/// which a strict-TLS edge requires.
	fn emit_custom_domains(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		region: &SmolStr,
		gateway: &ResourceDef<AwsApigatewayv2ApiDetails>,
		stage: &ResourceDef<AwsApigatewayv2StageDetails>,
	) -> Result {
		let Some(primary) = self.dns.first() else {
			return Ok(());
		};
		let sans = self.dns[1..]
			.iter()
			.map(|dns| dns.authority().clone())
			.collect::<Vec<_>>();
		let cert = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("cert")),
			AwsAcmCertificateDetails {
				domain_name: Some(primary.authority().clone()),
				validation_method: Some("DNS".into()),
				// Always declared, even when empty: the field is Optional +
				// Computed, so omitting it makes tofu keep the prior cert's SANs
				// rather than shrink the domain set.
				subject_alternative_names: Some(sans),
				region: Some(region.clone()),
				..default()
			},
		);

		// One validation record per authority. `domain_validation_options` is an
		// unordered set, so each record selects its option by `domain_name`,
		// trimming the trailing dot the dns providers reject; the raw fqdn is kept
		// for ACM matching.
		let dvo = cert.field("domain_validation_options");
		let mut validation_addresses = Vec::new();
		let mut validation_fqdns = Vec::new();
		for dns in &self.dns {
			let authority = dns.authority();
			let select = |attr: &str, trim: bool| {
				let value = format!("o.{attr}");
				let value = if trim {
					format!("trimsuffix({value}, \".\")")
				} else {
					value
				};
				format!(
					"${{[for o in {dvo} : {value} if o.domain_name == \"{authority}\"][0]}}"
				)
			};
			validation_addresses.push(SmolStr::from(dns.emit_cname(
				stack,
				config,
				&self.build_label(&format!(
					"cert-validation-{}",
					authority.replace('.', "-")
				)),
				&select("resource_record_name", true),
				&select("resource_record_value", true),
			)?));
			validation_fqdns
				.push(SmolStr::from(select("resource_record_name", false)));
		}

		let cert_validation = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("cert-validation")),
			AwsAcmCertificateValidationDetails {
				certificate_arn: cert.field_ref("arn").into(),
				validation_record_fqdns: Some(validation_fqdns),
				depends_on: Some(validation_addresses),
				region: Some(region.clone()),
				..default()
			},
		);
		config.add_resource(&cert)?.add_resource(&cert_validation)?;
		// SANs are immutable, so changing the domain set replaces the cert; create
		// the replacement (and validate it) before destroying the old one, so no
		// custom domain ever references a torn-down certificate.
		config.set_lifecycle(
			"aws_acm_certificate",
			cert.ident().label(),
			json!({ "create_before_destroy": true }),
		)?;

		// The gateway stage is named, not referenced, by the mapping, so the
		// dependency is declared rather than inferred.
		let stage_address =
			format!("aws_apigatewayv2_stage.{}", stage.ident().label());
		for dns in &self.dns {
			let authority = dns.authority();
			let suffix = authority.replace('.', "-");
			let domain = ResourceDef::new_secondary(
				stack.resource_ident(
					self.build_label(&format!("domain-{suffix}")),
				),
				AwsApigatewayv2DomainNameDetails {
					domain_name: authority.clone(),
					region: Some(region.clone()),
					domain_name_configuration: Some(vec![
						AwsApigatewayv2DomainNameResourceBlockTypeDomainNameConfiguration {
							// the validated arn, so the domain waits for issuance
							certificate_arn: cert_validation
								.field_ref("certificate_arn")
								.into(),
							endpoint_type: "REGIONAL".into(),
							security_policy: "TLS_1_2".into(),
							..default()
						},
					]),
					..default()
				},
			);
			let mapping = ResourceDef::new_secondary(
				stack.resource_ident(
					self.build_label(&format!("domain-mapping-{suffix}")),
				),
				AwsApigatewayv2ApiMappingDetails {
					api_id: gateway.field_ref("id").into(),
					domain_name: domain.field_ref("id").into(),
					stage: "$default".into(),
					region: Some(region.clone()),
					depends_on: Some(vec![stage_address.clone().into()]),
					..default()
				},
			);
			config.add_resource(&domain)?.add_resource(&mapping)?;
			dns.emit(
				stack,
				config,
				&self.build_label(&format!("dns-{suffix}")),
				&domain.field_ref(
					"domain_name_configuration[0].target_domain_name",
				),
			)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The terraform json for the given block.
	fn build_json(block: &LambdaBlock) -> String {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mut config = deployment.create_config(&stack);
		let mut world = World::new();
		block
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&deployment,
				&default(),
				&mut config,
			)
			.unwrap();
		config.to_json().to_string()
	}

	/// The function must carry `BEET_STAGE` so the deployed runtime reports the
	/// stage it is actually running in. Asserted against a named stage, since the
	/// `dev` default renders to nothing.
	#[beet_core::test]
	fn injects_beet_stage_env() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let stack = stack.with_stage("prod");
		let mut config = deployment.create_config(&stack);
		let mut world = World::new();
		LambdaBlock::default()
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&deployment,
				&default(),
				&mut config,
			)
			.unwrap();
		config
			.to_json()
			.to_string()
			.xpect_contains("\"BEET_STAGE\":\"prod\"");
	}

	/// The default block answers only on the gateway's `execute-api` endpoint, so
	/// it emits no certificate, no custom domain and no record at all.
	#[beet_core::test]
	fn no_dns_emits_no_custom_domain() {
		build_json(&LambdaBlock::default())
			.as_str()
			.xpect_contains("aws_apigatewayv2_api")
			.xnot()
			.xpect_contains("aws_acm_certificate")
			.xnot()
			.xpect_contains("aws_apigatewayv2_domain_name")
			.xnot()
			.xpect_contains("cloudflare_dns_record");
	}

	/// An authority is a custom domain, not a record at the default endpoint: the
	/// gateway routes by `Host`, so a bare record would be answered with a 403.
	/// The public record therefore points at the domain's `target_domain_name`.
	#[beet_core::test]
	fn dns_emits_custom_domain_mapped_to_the_default_stage() {
		let json = build_json(
			&LambdaBlock::default().with_dns(
				DnsProvider::cloudflare("example.org", "zone123")
					.with_proxied(true),
			),
		);
		json.as_str()
			.xpect_contains("aws_acm_certificate")
			.xpect_contains("aws_acm_certificate_validation")
			.xpect_contains("\"validation_method\":\"DNS\"")
			// declared even when empty, so a later shrink is tracked rather than
			// leaving a stale multi-SAN cert
			.xpect_contains("\"subject_alternative_names\":[]")
			.xpect_contains("aws_apigatewayv2_domain_name")
			.xpect_contains("\"endpoint_type\":\"REGIONAL\"")
			.xpect_contains("aws_apigatewayv2_api_mapping")
			.xpect_contains("\"stage\":\"$default\"")
			.xpect_contains("target_domain_name")
			.xpect_contains("zone123")
			// the site record rides the edge
			.xpect_contains("\"proxied\":true");
		// the acm validation record must stay unproxied, or the CA cannot read it
		json.matches("\"proxied\":false").count().xpect_eq(1);
		// the record points at the custom domain, never at the gateway's default
		// endpoint, which answers no other host
		json.as_str()
			.xpect_contains("domain_name_configuration[0].target_domain_name}");
	}

	/// Several authorities share one certificate (the rest as SANs) but each gets
	/// its own custom domain, mapping and record.
	#[beet_core::test]
	fn multiple_dns_emits_a_domain_each_on_one_cert() {
		let json = build_json(
			&LambdaBlock::default()
				.with_dns(DnsProvider::cloudflare("example.org", "z"))
				.with_dns(DnsProvider::cloudflare("www.example.org", "z")),
		);
		json.as_str()
			.xpect_contains(
				"\"subject_alternative_names\":[\"www.example.org\"]",
			)
			.xpect_contains("domain_example_org")
			.xpect_contains("domain_www_example_org")
			.xpect_contains("domain_mapping_example_org")
			.xpect_contains("domain_mapping_www_example_org");
		json.matches("\"aws_acm_certificate\"").count().xpect_eq(1);
		// one custom domain each, counted by the config block only a domain has
		json.matches("\"domain_name_configuration\"")
			.count()
			.xpect_eq(2);
	}

	// drives the native tofu Project, so it cannot compile for wasm
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 120000)]
	#[ignore = "very slow"]
	async fn validate() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let block = LambdaBlock::default();
		let mut config = deployment.create_config(&stack);
		let mut world = World::new();
		block
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&deployment,
				&default(),
				&mut config,
			)
			.unwrap();
		let project = terra::Project::new(stack, deployment, config);
		project.validate().await.unwrap();
	}
}
