use crate::bindings::AwsIamRoleDetails;
use crate::bindings::AwsIamRolePolicyAttachmentDetails;
use crate::bindings::AwsLambdaFunctionDetails;
use crate::bindings::AwsS3BucketDetails;
use crate::bindings::aws;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::json;

/// Opinionated terraform configuration for a standard web application:
/// - serverless lambda function
/// - html bucket
/// - assets bucket
/// - optional dns configuration
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
#[component(on_add=on_add)]
pub struct LambdaStack {
	dns: Option<DnsProvider>,
	/// specify the aws region for the
	/// buckets and lambda function
	region: Option<SmolStr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DnsProvider {
	Cloudflare { authority: SmolStr },
	Aws { authority: SmolStr },
}
impl DnsProvider {
	pub fn authority(&self) -> &str {
		match self {
			DnsProvider::Cloudflare { authority } => authority,
			DnsProvider::Aws { authority } => authority,
		}
	}
}


impl LambdaStack {
	fn region(&self) -> &str {
		self.region.as_deref().unwrap_or(aws::region::DEFAULT)
	}

	fn html_bucket_name(cx: &StackContext) -> terra::Ident {
		cx.resource_ident("html")
	}
	fn assets_bucket_name(cx: &StackContext) -> terra::Ident {
		cx.resource_ident("assets")
	}

	#[cfg(feature = "stack_lambda_rt")]
	pub async fn html_bucket(&self, cx: &StackContext) -> Bucket {
		self.bucket(Self::html_bucket_name(cx)).await
	}

	#[cfg(feature = "stack_lambda_rt")]
	async fn bucket(&self, ident: terra::Ident) -> Bucket {
		let provider = S3Provider::create_with_region(&self.region()).await;
		let slug = ident.primary_identifier();
		Bucket::new(provider, slug)
	}

	fn spawn(&self, mut commands: Commands, root: Entity) {
		// default to s3 backend
		commands
			.entity(root)
			.insert_if_new(Stack::new(S3Backend::default()));

		let html_bucket = commands.spawn((ChildOf(root),));

		let html_bucket = AwsS3BucketDetails {
			force_destroy: Some(true),
			region: Some(self.region().into()),
			..default()
		};
	}
}


fn on_add(mut world: DeferredWorld, cx: HookContext) {
	let stack = world
		.entity(cx.entity)
		.get::<LambdaStack>()
		.unwrap()
		.clone();
}


fn terra_config(
	cx: &StackContext,
	entity: &mut EntityWorldMut,
) -> Result<terra::Config> {
	entity
		.with_query::<(&Stack, &LambdaStack), _>(|(stack, lambda)| {
			let region = lambda.region();

			let html_bucket = ResourceDef::new_primary(
				LambdaStack::html_bucket_name(cx),
				AwsS3BucketDetails {
					force_destroy: Some(true),
					region: Some(region.into()),
					..default()
				},
			);

			let assets_bucket = ResourceDef::new_primary(
				LambdaStack::assets_bucket_name(cx),
				AwsS3BucketDetails {
					force_destroy: Some(true),
					region: Some(region.into()),
					..default()
				},
			);

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

			let lambda_policy = ResourceDef::new_secondary(
				cx.resource_ident("lambda_basic_policy"),
				AwsIamRolePolicyAttachmentDetails{
					policy_arn:
					"arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
						.into(),
						role: lambda_role.field("name").into(),
					..default()
				},
			);

			// just returns the string, ie for foo: String returns "foo"

			let lambda_function = ResourceDef::new_primary(
				cx.resource_ident("router"),
				AwsLambdaFunctionDetails {
					runtime: Some("provided.al2023".into()),
					handler: Some("bootstrap".into()),
					filename: Some("lambda.zip".into()),
					role: lambda_role.field("name").into(),
					timeout: Some(180),
					memory_size: Some(1024),
					source_code_hash: Some(default()),
					..default()
				},
			);

			let mut config = terra::Config::default()
				.with_backend(stack.backend())
				.with_resource(&html_bucket)
				.with_resource(&assets_bucket)
				.with_resource(&lambda_role)
				.with_resource(&lambda_policy)
				.with_resource(&lambda_function);

			if let Some(dns) = &lambda.dns {
				//todo gateway
				match dns {
					DnsProvider::Cloudflare { authority } => todo!(),
					DnsProvider::Aws { authority } => todo!(),
				}
			}

			config
		})?
		.xok()
}


#[derive(Component)]
pub struct BucketDef {
	label: SmolStr,
	details: AwsS3BucketDetails,
}

impl BucketDef {}
