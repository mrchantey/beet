use beet::prelude::*;
use beet_web::prelude::*;
use bevy::prelude::*;
use sweet::*;


#[sweet_test]
pub fn works() -> Result<()> {
	append_html_for_tests();

	BeetWebApp::default()
		.with_test_container()
		.with_behavior(
			bee_bundle(),
			Translate::new(Vec3::new(-0.1, 0., 0.)),
		)
		.run_forever()?;

	Ok(())
}
