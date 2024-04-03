use beet::prelude::*;
use beet_web::prelude::*;
use bevy::prelude::*;
use std::time::Duration;
use sweet::*;


#[sweet_test]
async fn works() -> Result<()> {
	append_html_for_tests();

	DomSim::<BeetWebNode> {
		bees: 1,
		flowers: 10,
		auto_flowers: Some(Duration::from_secs(1)),
		..default()
	}
	.with_test_container(render_container())
	.with_node(forage())
	.run_forever()?;
	Ok(())
}
