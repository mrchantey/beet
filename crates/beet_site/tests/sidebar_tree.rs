#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;


#[sweet::test]
async fn works() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, RouterPlugin, server_plugin));
	app.init().update();
	app.world_mut()
		.run_system_cached_with(
			CollectSidebarNode::collect,
			CollectSidebarNode {
				include_filter: GlobFilter::default()
					.with_include("/")
					.with_include("/docs*")
					.with_include("/blog*")
					.with_include("/design*"),
				expanded_filter: GlobFilter::default().with_include("/docs"),
			},
		)
		.unwrap()
		.paths()
		.len()
		.xpect_greater_than(10);
}
