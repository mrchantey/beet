// use beet::graph::BeetNode;
use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::core::Name;
use bevy::math::Vec3;


pub fn forage() -> BeetNode {
	let awareness_radius = 0.5;

	(
		Name::new("Selector"),
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
		)
}
