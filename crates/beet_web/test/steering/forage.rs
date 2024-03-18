use beet_core::prelude::*;
use beet_web::prelude::*;
use bevy::prelude::*;
use sweet::*;


#[sweet_test]
async fn works() -> Result<()> {
	append_html_for_tests();

	AppOptions {
		bees: 1,
		flowers: 10,
		auto_flowers: Some(1000),
		..default()
	}
	.with_graph(forage())
	.run();
	Ok(())
}
