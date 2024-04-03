use super::*;
use crate::prelude::*;
use beet_ecs::exports::ScheduleLabel;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;

// action_list!(CoreNode, [
// ]);

#[derive(Debug, Clone, ActionList)]
#[actions(
	//core
		Translate,
	//steer
		Seek,
		Wander,
		FindSteerTarget,
		DespawnSteerTarget,
		ScoreSteerTarget,
		SucceedOnArrive,
	//ecs
		EcsNode
	)]
#[components(
	//force bundle
	Mass, 
	Velocity, 
	Impulse, 
	Force,
	//steer bundle
	MaxForce,
	MaxSpeed,
	ArriveRadius,
	WanderParams,
)]
pub struct CoreNode;
