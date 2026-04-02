use crate::bindings::AwsS3BucketDetails;
use crate::bindings::aws;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

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
		let slug = slug.to_kebab_case();
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
	// lambda_stack(world.commands(), cx.entity, stack);
}

// fn resource_name(label: &str) -> Vec<PathPatternSegment> {
// 	vec![
// 		PathPatternSegment::new_required("app_name"),
// 		PathPatternSegment::new_static(label),
// 	]
// }


pub trait ResourceTool {
	fn definition(&self) -> String;
	#[cfg(feature = "tokens")]
	fn rust_type() -> proc_macro2::TokenStream;
}


#[derive(Component)]
pub struct BucketDef {
	label: SmolStr,
}
