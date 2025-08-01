#![allow(unused)]
use crate::prelude::*;
use beet::prelude::*;


#[template]
pub fn BeetSidebarLayout(world: &mut World) -> Result<impl Bundle> {
	let sidebar_nodes = world
		.run_system_cached_with(
			CollectSidebarNode::collect,
			CollectSidebarNode {
				include_filter: GlobFilter::default()
					.with_include("/docs/*")
					.with_include("/blog/*"),
				expanded_filter: GlobFilter::default().with_include("/docs/"),
			},
		)?
		.children;
	// let sidebar_nodes = todo!("get route tree");
	rsx! {
		<slot/>

	}
	.xok()
}
