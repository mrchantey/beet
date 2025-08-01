#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;

#[sweet::test]
async fn docs() {
	let res = Router::new(|app: &mut App| {
		app.insert_resource(TemplateFlags::None)
			.world_mut()
			.spawn(routes());
	})
	.oneshot("/docs")
	.await;
	println!("{:?}", res);
}
