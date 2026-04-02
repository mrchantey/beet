use crate::bindings::aws;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

#[derive(Debug, Clone, Component)]
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
	#[cfg(feature = "stack_lambda_rt")]
	pub async fn html_bucket(&self, cx: &StackContext) -> Bucket {
		let region = self
			.region
			.clone()
			.unwrap_or_else(|| aws::region::DEFAULT.into());

		let provider = S3Provider::create_with_region(&region).await;
		let slug = Self::html_bucket_name(cx).to_kebab_case();

		Bucket::new(provider, slug)
	}

	fn html_bucket_name(cx: &StackContext) -> Slug {
		Slug::new(vec![
			cx.app_name().clone(),
			cx.stage().clone(),
			"buckets".into(),
			"html".into(),
		])
	}
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	let stack = world
		.entity(cx.entity)
		.get::<LambdaStack>()
		.unwrap()
		.clone();
	lambda_stack(world.commands().entity(cx.entity), stack);
}


// fn resource_name(label: &str) -> Vec<PathPatternSegment> {
// 	vec![
// 		PathPatternSegment::new_required("app_name"),
// 		PathPatternSegment::new_static(label),
// 	]
// }

fn lambda_stack(mut commands: EntityCommands, _stack: LambdaStack) {
	commands.insert_if_new(Stack::new(S3Backend::default()));
}

pub trait ResourceTool {
	fn definition(&self) -> String;
	#[cfg(feature = "tokens")]
	fn rust_type() -> proc_macro2::TokenStream;
}
