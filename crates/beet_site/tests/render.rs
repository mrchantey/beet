#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;

#[beet::test]
async fn home() {
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.exchange_str("/")
		.await
		.xnot()
		.xpect_contains("fn Counter(initial: u32)");
}
#[beet::test]
async fn help() {
	// test with ssr
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.exchange_str("/?help")
		.await
		.xnot()
		.xpect_contains("Welcome to Beet!");
	// test with ssg
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssg)
		.spawn(beet_site_router())
		.exchange_str("/?help")
		.await
		.xnot()
		.xpect_contains("Welcome to Beet!");
}

#[beet::test]
async fn docs() {
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.exchange_str("/docs")
		.await
		.xpect_contains("docs")
		// nav should be scoped style, ie nav[beet-style-id..]
		.xnot()
		.xpect_contains("nav {");
}


#[beet::test]
async fn article_layout() {
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.exchange_str("/blog/post-1")
		.await
		.xpect_contains(r#"<meta charset="UTF-8"/>"#);
}
#[beet::test]
async fn multiple_calls() {
	let mut world = RouterPlugin::world();
	let mut entity = world
		.with_resource(pkg_config!())
		// .with_resource(RenderMode::Ssr)
		.spawn(beet_site_router());
	entity
		.exchange_str("/")
		.await
		.xpect_contains(r#"<meta charset="UTF-8"/>"#);
	entity
		.exchange(
			Request::post("/analytics")
				.with_json_body(&serde_json::json! {{"foo":"bar"}})
				.unwrap(),
		)
		.await
		.into_result()
		.await
		.xpect_ok();
	entity
		.exchange(
			Request::post("/analytics")
				.with_json_body(&serde_json::json! {{"foo":"bar"}})
				.unwrap(),
		)
		.await
		.into_result()
		.await
		.xpect_ok();
	entity
		.exchange(
			Request::post("/analytics")
				.with_json_body(&serde_json::json! {{"foo":"bar"}})
				.unwrap(),
		)
		.await
		.into_result()
		.await
		.xpect_ok();
	entity
		.exchange_str("/")
		.await
		.xpect_contains(r#"<meta charset="UTF-8"/>"#);
	entity
		.exchange_str("/")
		.await
		.xpect_contains(r#"<meta charset="UTF-8"/>"#);
	entity
		.exchange_str("/")
		.await
		.xpect_contains(r#"<meta charset="UTF-8"/>"#);
}

#[beet::test]
#[ignore = "flaky: sometimes beet_site sometimes beet"]
async fn correct_title() {
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.exchange_str("/blog/post-1")
		.await
		.xpect_contains(r#"<title>Beet</title>"#);
}
