#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;

#[sweet::test]
async fn works() {
	let foo = DocsMeta {
		title: Some("Beet Site".into()),
		description: Some("A very bevy metaframework".into()),
		draft: false,
		sidebar: SidebarInfo {
			label: Some("Beet Site".into()),
			..Default::default()
		},
	};

	let ron = beet::exports::ron::to_string(&foo).unwrap();
	println!("Ron: {}", ron);

	// let router =
	// 	AppRouter::test().add_plugins((PagesPlugin, DocsPlugin, ActionsPlugin));

	// for route in route_path_tree().flatten().iter() {
	// 	router
	// 		.render_route(&RouteInfo::get(&route.0))
	// 		.await
	// 		.unwrap();
	// }

	// .render_route(&"/".into())
	// .await
	// .unwrap()
	// .xpect()
	// .to_contain("A very bevy metaframework");
}
