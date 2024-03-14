use super::*;
use crate::prelude::*;
use beet_ecs::exports::ScheduleLabel;
use beet_ecs::prelude::*;
use bevy_app::App;
use bevy_reflect::TypeRegistry;


// action_list!(CoreNode, [
// 	//core
// 	Translate,
// 	//steer
// 	Seek,
// 	Wander,
// 	FindSteerTarget,
// 	DespawnSteerTarget,
// 	ScoreSteerTarget,
// 	SucceedOnArrive,
// 	//ecs
// 	EcsNode
// ]);

#[derive(Debug, Clone)]
pub struct CoreNode;

impl ActionSystems for CoreNode {
	fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone) {
		//core
		Translate::add_systems(app, schedule.clone());
		//steer
		Seek::add_systems(app, schedule.clone());
		Wander::add_systems(app, schedule.clone());
		SetAgentOnRun::<Velocity>::add_systems(app, schedule.clone());
		FindSteerTarget::add_systems(app, schedule.clone());
		DespawnSteerTarget::add_systems(app, schedule.clone());
		ScoreSteerTarget::add_systems(app, schedule.clone());
		SucceedOnArrive::add_systems(app, schedule.clone());

		EcsNode::add_systems(app, schedule.clone());
	}
}


impl ActionTypes for CoreNode {
	fn register(registry: &mut TypeRegistry) {
		//core
		Translate::register(registry);
		//steer
		Seek::register(registry);
		Wander::register(registry);
		SetAgentOnRun::<Velocity>::register(registry);
		FindSteerTarget::register(registry);
		DespawnSteerTarget::register(registry);
		ScoreSteerTarget::register(registry);
		SucceedOnArrive::register(registry);

		EcsNode::register(registry);
	}
}
