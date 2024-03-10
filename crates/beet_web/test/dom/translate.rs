use beet::prelude::*;
use beet_web::prelude::*;
use bevy_math::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();

	AppOptions::default()
		.with_graph(BehaviorTree::new(Translate::new(Vec3::new(-0.1, 0., 0.))))
		.run();

	Ok(())
}
