#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;


#[beet::test]
async fn works() {
	let mut world = server_plugin.into_world();

	let endpoints = EndpointTree::endpoints_from_bundle_func(
		&mut world,
		beet_site_endpoints,
	)
	.unwrap();
	let endpoint_tree = EndpointTree::from_endpoints(endpoints).unwrap();

	let root = world
		.run_system_cached_with(
			CollectSidebarNode::collect,
			(
				CollectSidebarNode {
					include_filter: GlobFilter::default()
						.with_include("/")
						.with_include("/docs*")
						.with_include("/blog*")
						.with_include("/design*"),
					expanded_filter: GlobFilter::default()
						.with_include("/docs"),
				},
				endpoint_tree,
			),
		)
		.unwrap();

	root.expanded.xpect_false();
	root.children[0].display_name.xpect_eq("Blog");
	root.children[0].expanded.xpect_false();
	root.children[1].display_name.xpect_eq("Docs");
	root.children[1].expanded.xpect_true();
}
