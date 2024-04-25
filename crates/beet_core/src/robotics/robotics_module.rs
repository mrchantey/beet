use super::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default, Clone, BeetModule)]
#[actions(
	SetAgentOnRun::<DualMotorValue>,
	DepthSensorScorer,
)]
#[components(DepthValue, DualMotorValue)]
pub struct RoboticsModule;
