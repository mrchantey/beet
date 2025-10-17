#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;

#[sweet::test]
// #[ignore]
async fn docs() {
	server_plugin
		.into_world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.await_event::<Insert, RouteServer>()
		.await
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
#[sweet::test]
async fn article_layout() {
	server_plugin
		.into_world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.await_event::<Insert, RouteServer>()
		.await
		.oneshot("/blog/post-1")
		.await
		.into_result()
		.await
		.unwrap()
		.text()
		.await
		.unwrap()
		.xpect_contains(r#"<meta charset="UTF-8"/><title>Beet</title>"#);
}
