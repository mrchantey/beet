#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;

#[sweet::test]
async fn home() {
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.oneshot_str("/")
		.await
		.xnot()
		.xpect_contains("fn Counter(initial: u32)");
}
#[sweet::test]
async fn help() {
	// test with ssr
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.oneshot_str("/?help")
		.await
		.xnot()
		.xpect_contains("Welcome to Beet!");
	// test with ssg - skip for now due to endpoint conflict
	// RouterPlugin::world()
	// 	.with_resource(pkg_config!())
	// 	.with_resource(RenderMode::Ssg)
	// 	.spawn(beet_site_router())
	// 	.oneshot_str("/?help")
	// 	.await
	// 	.xnot()
	// 	.xpect_contains("Welcome to Beet!");
}

#[sweet::test]
async fn docs() {
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.oneshot_str("/docs")
		.await
		.xpect_contains("docs")
		// nav should be scoped style, ie nav[beet-style-id..]
		.xnot()
		.xpect_contains("nav {");
}


#[sweet::test]
async fn article_layout() {
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.oneshot_str("/blog/post-1")
		.await
		.xpect_contains(r#"<meta charset="UTF-8"/>"#);
}

#[sweet::test]
#[ignore = "flaky: sometimes beet_site sometimes beet"]
async fn correct_title() {
	RouterPlugin::world()
		.with_resource(pkg_config!())
		.with_resource(RenderMode::Ssr)
		.spawn(beet_site_router())
		.oneshot_str("/blog/post-1")
		.await
		.xpect_contains(r#"<title>Beet</title>"#);
}
