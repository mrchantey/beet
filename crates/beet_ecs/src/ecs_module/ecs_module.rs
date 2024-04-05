use crate::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;

#[derive(Debug, Clone, ActionList)]
#[actions(
	// constants
	SetOnStart::<Score>,
	InsertOnRun::<RunResult>,
	// utility
	EmptyAction,
	Repeat,
	SucceedInDuration,
	// selectors
	SequenceSelector,
	FallbackSelector,
	ScoreSelector
)]
#[components(
	// running
	Running,
	RunTimer,
	RunResult,
	// graph
	Parent,
	Children,
	BeetRoot,
	NeedsParentRoot,
	ParentRoot,
	ActionTarget,
	// bevy
	Name,
	Transform,
	GlobalTransform,
)]
pub struct EcsModule;
