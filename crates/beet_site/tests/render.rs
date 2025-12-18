#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;

#[sweet::test]
async fn test_layouts_series() {
	let mut world = server_plugin.into_world();

	world
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.await_event::<Insert, Router>()
		.await;

	docs(&mut world).await;
	article_layout(&mut world).await;
	// correct_title(&mut world).await;
}
// #[ignore]
async fn docs(world: &mut World) {
	world
		.oneshot("/docs")
		.await
		.into_result()
		.await
		.unwrap()
		.text()
		.await
		.unwrap()
		.xpect_contains("docs")
		// nav should be scoped style, ie nav[beet-style-id..]
		.xnot()
		.xpect_contains("nav {");
}
async fn article_layout(world: &mut World) {
	world
		.oneshot("/blog/post-1")
		.await
		.into_result()
		.await
		.unwrap()
		.text()
		.await
		.unwrap()
		.xpect_contains(r#"<meta charset="UTF-8"/>"#);
}
#[allow(unused)]
async fn correct_title(world: &mut World) {
	world
		.oneshot("/blog/post-1")
		.await
		.into_result()
		.await
		.unwrap()
		.text()
		.await
		.unwrap()
		.xpect_contains(r#"<title>Beet</title>"#);
}
