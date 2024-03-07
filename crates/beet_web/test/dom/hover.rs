use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	run(GameConfig {
		graph: BehaviorTree::new(Hover::new(1., 0.01)).into_action_graph(),
		..Default::default()
	});
	Ok(())
}
