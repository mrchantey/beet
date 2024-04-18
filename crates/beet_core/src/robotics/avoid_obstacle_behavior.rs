use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;


pub fn avoid_obstacle_behavior() -> BeetBuilder {
	let threshold_dist = 0.5; //meters

	BeetBuilder::new((
		Name::new("Avoid Obstacles"),
		LogNameOnRun,
		ScoreSelector::default(),
	))
	.child((
		Name::new("Drive Forward"),
		LogNameOnRun,
		RootIsTargetAgent,
		Score::Weight(0.5),
		SetAgentOnRun(DualMotorValue::splat(MotorValue::forward_max())),
	))
	.child((
		Name::new("Turn Right"),
		LogNameOnRun,
		RootIsTargetAgent,
		Score::default(),
		DepthSensorScorer::new(threshold_dist),
		SetAgentOnRun(DualMotorValue::new(
			MotorValue::forward_max(),
			MotorValue::backward_max(),
		)),
	))
}
