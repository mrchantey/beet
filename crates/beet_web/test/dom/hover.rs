use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	let mut relay = Relay::default();

	BeeGame::create_bee_pub(&mut relay)
		.push(&BehaviorTree::new(Hover::new(1., 0.01)).into_action_graph())?;
	BeeGame::create_flower_pub(&mut relay).push(&())?;

	run(relay);
	Ok(())
}