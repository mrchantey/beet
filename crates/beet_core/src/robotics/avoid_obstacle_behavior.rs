use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;


pub fn avoid_obstacle_behavior(world: &mut World) -> Entity {
	let threshold_dist = 0.5; //meters

	world
		.spawn((
			Name::new("Avoid Obstacles"),
			Running,
			BeetRoot,
			ScoreSelector::default(),
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Drive Forward"),
				RootIsTargetAgent,
				Score::Weight(0.5),
				SetAgentOnRun(DualMotorValue::splat(MotorValue::forward_max())),
			));

			parent.spawn((
				Name::new("Turn Right"),
				RootIsTargetAgent,
				Score::default(),
				DepthSensorScorer::new(threshold_dist),
				SetAgentOnRun(DualMotorValue::new(
					MotorValue::forward_max(),
					MotorValue::backward_max(),
				)),
			));
		})
		.id()
}
