use beet::prelude::*;
use beet_web::prelude::*;
use bevy_math::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	run(GameConfig {
		graph: BehaviorTree::new(Translate::new(Vec3::new(-0.1, 0., 0.)))
			.into_action_graph(),
		..Default::default()
	});
	Ok(())
}
