#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;

#[sweet::test]
async fn docs() {
	let mut app = App::new();

	app.add_plugins(RouterPlugin);
	app.world_mut().spawn(routes());
	app.insert_resource(TemplateFlags::None).init().update();
	let _res = Router::oneshot(app.world_mut(), "/docs").await;
	// println!("{:?}", res);
}
