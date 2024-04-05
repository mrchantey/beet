use std::borrow::Cow;

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
	RandomizePosition,
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
pub struct RenderText(pub Cow<'static, str>);

impl RenderText{
	pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
		Self(text.into())
	}
}

#[derive(Default)]
pub struct CorePlugin;

impl Plugin for CorePlugin {
	fn build(&self, app: &mut App) {
		app /*-*/
			.add_systems(Update, auto_spawn.before(PreTickSet))
			.add_systems(Update, randomize_position.in_set(PreTickSet))
		/*-*/;
	}
}
