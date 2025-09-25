#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;

#[sweet::test]
// #[ignore = "changes too often"]
async fn docs() {
	// let res = Router::new_bundle(routes_bundle)
	// 	.oneshot("/docs")
	// 	.await
	// 	// .text()
	// 	// .await
	// 	.unwrap();
	// println!("res: {res:?}");
	// empty without snippets.ron?

	Router::new(server_routes_plugin)
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.oneshot("/docs")
		.await
		.into_result()
		.await
		.unwrap()
		.text()
		.await
		.unwrap()
		.xpect_contains("docs");
}
