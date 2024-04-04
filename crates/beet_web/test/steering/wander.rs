use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	BeetWebApp::default()
		.with_test_container()
		.with(spawn_bee)
		.with_node(Wander)?
		.run_forever()?;

	Ok(())
}
