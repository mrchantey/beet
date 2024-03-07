use beet::prelude::*;
use beet_web::prelude::*;
use bevy_math::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	// expect(true).to_be_false()?;
	let relay = Relay::default();
	let graph = BehaviorTree::new(
		vec![Translate::new(Vec3::new(-0.1, 0., 0.)).into()].into(),
	)
	.into_action_graph();

	let game = GameConfig {
		relay: relay.clone(),
		graph,
		// flower: false,
		flower: true,
	};


	run(game);

	Ok(())
}
