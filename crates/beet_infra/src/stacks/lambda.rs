use crate::bindings::AwsIamRoleDetails;
use crate::bindings::AwsS3BucketDetails;
use crate::bindings::aws;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::json;

#[derive(Debug, Clone, Component, Serialize, Deserialize)]
#[component(on_add=on_add)]
pub struct LambdaStack {
	/// prepended to resources to differentiate
	/// this lambda stack, useful when multiple
	/// lambda stacks per app, though.. why would you..
	prefix: Option<SmolStr>,
	authority: Option<SmolStr>,
	/// specify the aws region for the
	/// buckets and lambda function
	region: Option<SmolStr>,
}




impl LambdaStack {
	fn region(&self) -> &str {
		self.region.as_deref().unwrap_or(aws::region::DEFAULT)
	}

	fn html_bucket_name(cx: &StackContext) -> Slug { cx.bucket_slug("html") }
	fn assets_bucket_name(cx: &StackContext) -> Slug {
		cx.bucket_slug("assets")
	}

	#[cfg(feature = "stack_lambda_rt")]
	pub async fn html_bucket(&self, cx: &StackContext) -> Bucket {
		self.bucket(Self::html_bucket_name(cx)).await
	}

	#[cfg(feature = "stack_lambda_rt")]
	async fn bucket(&self, slug: Slug) -> Bucket {
		let provider = S3Provider::create_with_region(&self.region()).await;
		let slug = slug.primary_identifier();
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
	let mut config = terra::Config::default();

	let stack =
		entity.with_query::<(&Stack, &LambdaStack), _>(|(stack, lambda)| {
			config.set_backend(stack.backend());
			let region = lambda.region();

			let html_bucket_slug = LambdaStack::html_bucket_name(cx);

			config.add_resource(
				html_bucket_slug.label(),
				&AwsS3BucketDetails {
					bucket: Some(html_bucket_slug.primary_identifier().into()),
					force_destroy: Some(true),
					region: Some(region.into()),
					..default()
				},
			);

			let assets_bucket_slug = LambdaStack::assets_bucket_name(cx);

			config.add_named_resource(
				&assets_bucket_slug,
				AwsS3BucketDetails {
					force_destroy: Some(true),
					region: Some(region.into()),
					..default()
				},
			);


			let lambda_role_slug = cx.iam_role_slug("lambda");

			config.add_named_resource(&lambda_role_slug, AwsIamRoleDetails {
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
			});

			todo!("for each field in a struct, generate a function foo_attr()->&'static str");
			// just returns the string, ie for foo: String returns "foo"

			// aws_iam_role.lambda_role.name
			// 1. resource-kind
			// 2. resource-label
			// 3. field-name
		})?;

	// config.add_resource(name, resource)

	config.xok()
}


#[derive(Component)]
pub struct BucketDef {
	label: SmolStr,
	details: AwsS3BucketDetails,
}

impl BucketDef {}
