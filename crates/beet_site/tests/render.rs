#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;
#[sweet::test]
async fn works() {
	let router =
		AppRouter::test().add_plugins((PagesPlugin, DocsPlugin, ActionsPlugin));

	// Ensure all routes build, including parsing their metadata.
	for route in route_path_tree().flatten().iter() {
		router
			.render_route(&RouteInfo::get(&route.0))
			.await
			.unwrap();
	}

	// check a route contains content
	router
		.render_route(&"/".into())
		.await
		.unwrap()
		.xpect()
		.to_contain("A very bevy metaframework");
}
