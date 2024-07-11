use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

/// A useful default behavior
pub fn forage_behavior(world: &mut World) -> Entity {
	let awareness_radius = 0.5;
	world
		.spawn((
			Name::new("Forage"),
			Repeat::default(),
			ScoreSelector::default(),
			FindSteerTarget::new("flower", awareness_radius),
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Wander"),
				Score::default(),
				SetOnSpawn(Score::Weight(0.5)),
				Wander::default(),
				InsertInDuration::<RunResult>::default(),
			));
			parent
				.spawn((
					Name::new("Seek"),
					Score::default(),
					SteerTargetScoreProvider::new(awareness_radius),
					SequenceSelector::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Go to flower"),
						Seek::default(),
						EndOnArrive { radius: 0.1 },
					));
					parent.spawn((
						Name::new("Wait 1 second"),
						SetAgentOnRun(Velocity(Vec3::ZERO)),
						InsertInDuration::<RunResult>::default(),
					));
					parent.spawn((
						Name::new("Collect flower"),
						InsertOnRun(RunResult::Success),
						DespawnSteerTarget::default(),
					));
				});
		})
		.id()
}
