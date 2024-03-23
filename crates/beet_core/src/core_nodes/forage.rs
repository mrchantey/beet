// use beet::graph::BeetNode;
use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::core::Name;
use bevy::math::Vec3;


pub fn forage() -> BeetNode {
	let awareness_radius = 0.5;

	(
		Name::new("Wander or seek"),
		Repeat::default(),
		ScoreSelector::default(),
		FindSteerTarget::new("flower", awareness_radius),
	)
		.child((
			Name::new("Wander"),
			Score::default(),
			SetOnStart(Score::Weight(0.5)),
			Wander::default(),
		))
		.child(
			(
				Name::new("Seek"),
				Score::default(),
				ScoreSteerTarget::new(awareness_radius),
				SequenceSelector::default(),
			)
				.child((
					Name::new("Go to flower"),
					Seek::default(),
					SucceedOnArrive { radius: 0.1 },
				))
				.child((
					Name::new("Wait 1 second"),
					SetAgentOnRun(Velocity(Vec3::ZERO)),
					SucceedInDuration::with_secs(1),
				))
				.child((
					Name::new("Collect flower"),
					InsertOnRun(RunResult::Success),
					DespawnSteerTarget::default(),
				)),
		)
}
