use beet::prelude::*;
use beet_web::prelude::*;
use bevy_math::Vec3;
use bevy_utils::prelude::default;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	let awareness_radius = 0.5;

	AppOptions {
		bees: 2,
		flowers: 10,
		auto_flowers: Some(1000),
		..default()
	}
	.with_graph(
		BehaviorTree::new((
			Repeat::default(),
			UtilitySelector::default(),
			FindSteerTarget::new("flower", awareness_radius),
		))
		.with_child((Wander::default(), ConstantScore::new(Score::Weight(0.5))))
		.with_child(
			BehaviorTree::new((
				SequenceSelector::default(),
				ScoreSteerTarget::new(awareness_radius),
			))
			.with_child((Seek::default(), SucceedOnArrive { radius: 0.1 }))
			.with_child((
				SetVelocity(Vec3::ZERO),
				SucceedInDuration::with_secs(1),
			))
			.with_child((
				SetRunResult::success(),
				DespawnSteerTarget::default(),
			)),
		),
	)
	.run();
	Ok(())
}
