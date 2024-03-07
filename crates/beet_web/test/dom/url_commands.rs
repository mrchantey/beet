use beet::prelude::*;
use beet_web::prelude::*;
use bevy_math::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	// expect(true).to_be_false()?;
	run(BehaviorTree::new(
		vec![Translate::new(Vec3::new(-0.1, 0., 0.)).into()].into(),
	)
	.into_action_graph());

	Ok(())
}
