#![allow(unused)]
use beet_core::prelude::*;
use beet_net::prelude::*;

#[derive(Debug, Clone, Component)]
#[component(on_add=on_add)]
pub struct LambdaStack;

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

pub fn lambda_stack(mut commands: EntityCommands, stack: LambdaStack) {
	// commands.insert(TofuStack::new(S3Backend::new(AwsS3BucketDetails {
	// 	..default()
	// })));
}
