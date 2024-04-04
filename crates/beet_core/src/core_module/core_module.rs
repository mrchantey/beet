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
		Hover,
		Translate,
		// force
		SetAgentOnRun::<Velocity>,
		//steer
		Seek,
		Wander,
		FindSteerTarget,
		DespawnSteerTarget,
		ScoreSteerTarget,
		SucceedOnArrive,
	//ecs
		EcsModule
	)]
#[components(
	AutoSpawn,
	//render
	RenderText,
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
pub struct CoreModule;


#[derive(Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct RenderText(pub String);
