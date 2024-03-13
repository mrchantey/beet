use beet::prelude::*;
use beet_web::prelude::*;
use bevy_math::Vec3;
use bevy_utils::prelude::default;
use sweet::*;


#[sweet_test]
async fn works() -> Result<()> {
	append_html_for_tests();
	let awareness_radius = 0.5;

	AppOptions {
		bees: 1,
		flowers: 10,
		auto_flowers: Some(1000),
		..default()
	}
	.with_graph(
		(
			Repeat::default(),
			UtilitySelector::default(),
			FindSteerTarget::new("flower", awareness_radius),
		)
			.child((
				Score::default(),
				// ConstantScore::new(Score::Weight(0.5)),
				ConstantScore::new(Score::Fail), // should be weight 0.5 but buggin out
				Wander::default(),
			))
			.child(
				(
					Score::default(),
					ScoreSteerTarget::new(awareness_radius),
					SequenceSelector::default(),
				)
					.child((Seek::default(), SucceedOnArrive { radius: 0.1 }))
					.child((
						SetVelocity(Vec3::ZERO),
						SucceedInDuration::with_secs(1),
					))
					.child((
						SetRunResult::success(),
						DespawnSteerTarget::default(),
					)),
			),
	)
	.run();
	Ok(())
}
