use beet_core::prelude::*;
use beet_ecs::prelude::*;
use beet_web::prelude::*;
use bevy::prelude::*;
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
				SetOnStart(Score::Weight(0.5)),
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
						SetAgentOnRun(Velocity(Vec3::ZERO)),
						SucceedInDuration::with_secs(1),
					))
					.child((
						InsertOnRun(RunResult::Success),
						DespawnSteerTarget::default(),
					)),
			),
	)
	.run();
	Ok(())
}
