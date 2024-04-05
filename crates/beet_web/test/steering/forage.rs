use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;


#[sweet_test]
async fn works() -> Result<()> {
	append_html_for_tests();

	BeetWebApp::default()
		.with_test_container()
		.with_bundle(flower_bundle())
		.with_bundle(flower_bundle())
		.with_bundle(flower_bundle())
		.with_bundle(flower_bundle())
		.with(flower_auto_spawn)
		.with_behavior(bee_bundle(), forage())
		.run_forever()?;
	Ok(())
}
