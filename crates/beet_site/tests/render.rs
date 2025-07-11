#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;

fn router() -> AppRouter {
	AppRouter::test().add_plugins((
		PagesPlugin,
		DocsPlugin,
		ActionsPlugin,
		BeetDesignMockupsPlugin,
	))
}


#[sweet::test]
async fn root() {
	let router = router();
	let index = router.render_route(&"/".into()).await.unwrap();
	// println!("{}", index);
	index.xref().xpect().to_contain("data-beet-dom-idx");
	index.xref().xpect().to_contain("The Unistack Metaframework");
}
#[sweet::test]
async fn mdx() {
	let router = router().set_bevy_plugins(|app: &mut App| {
		app.insert_resource(TemplateFlags::All);
	});
	let _index = router.render_route(&"/docs".into()).await.unwrap();
	// println!("{}", _index);
}


#[sweet::test]
#[ignore = "reason"]
async fn render_all() {
	let router = router();

	// // Ensure all routes build, including parsing their metadata.
	for route in route_path_tree().flatten().iter() {
		router
			.render_route(&RouteInfo::get(&route.0))
			.await
			.unwrap();
	}
}
