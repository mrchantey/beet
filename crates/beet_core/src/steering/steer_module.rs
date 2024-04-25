use super::*;
use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default, Clone, BeetModule)]
#[actions(
		Seek,
		Wander,
		Separate::<GroupSteerAgent>,
		Align::<GroupSteerAgent>,
		Cohere::<GroupSteerAgent>,
		SucceedOnArrive,
		FindSteerTarget,
		ScoreSteerTarget,
		DespawnSteerTarget,
	)]
#[components(SteerTarget, MaxForce, MaxSpeed, ArriveRadius, WanderParams)]
#[bundles(SteerBundle)]
/// Actions for target and group based steering.
pub struct SteerModule;
