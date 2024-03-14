use crate::prelude::*;
use bevy_app::App;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_reflect::TypeRegistry;

// action_list!(EcsNode, [
// 	SetOnStart::<Score>,
// 	EmptyAction,
// 	FallbackSelector,
// 	Repeat,
// 	SetRunResult,
// 	SequenceSelector,
// 	SucceedInDuration,
// 	UtilitySelector
// ]);



#[derive(Debug, Clone)] // must be debug and clone to be ActionList
pub struct EcsNode;

impl ActionSystems for EcsNode {
	fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone) {
		// constants
		SetOnStart::<Score>::add_systems(app, schedule.clone());
		InsertOnRun::<RunResult>::add_systems(app, schedule.clone());
		// utility
		EmptyAction::add_systems(app, schedule.clone());
		Repeat::add_systems(app, schedule.clone());
		SucceedInDuration::add_systems(app, schedule.clone());
		// selectors
		SequenceSelector::add_systems(app, schedule.clone());
		FallbackSelector::add_systems(app, schedule.clone());
		UtilitySelector::add_systems(app, schedule.clone());
	}
}
impl ActionTypes for EcsNode {
	fn register(registry: &mut TypeRegistry) {
		SetOnStart::<Score>::register(registry);
		InsertOnRun::<RunResult>::register(registry);
		// utility
		EmptyAction::register(registry);
		Repeat::register(registry);
		SucceedInDuration::register(registry);
		// selectors
		SequenceSelector::register(registry);
		FallbackSelector::register(registry);
		UtilitySelector::register(registry);
	}
}
