use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;


#[sweet_test]
async fn works() -> Result<()> {
	append_html_for_tests();

	BeetWebApp::default()
		.with(spawn_bee)
		.with(spawn_flower)
		.with(spawn_flower)
		.with(spawn_flower)
		.with(spawn_flower)
		.with(flower_auto_spawn)
		.with_test_container()
		.with_node(forage())?
		.run_forever()?;
	Ok(())
}
