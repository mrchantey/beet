#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_site::prelude::*;
use sweet::prelude::*;

#[sweet::test]
#[ignore = "changes too often"]
async fn docs() {
	Router::new(|app: &mut App| {
		app.insert_resource(TemplateFlags::None)
			.world_mut()
			.spawn(routes());
	})
	.oneshot("/docs")
	.await
	.text()
	.await
	.unwrap()
	.xpect()
	.to_be_snapshot();
}
