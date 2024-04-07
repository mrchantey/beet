use crate::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;

#[derive(Debug, Clone, BeetModule)]
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
	// transform bundle
	Transform,
	GlobalTransform,
)]
#[bundles(TransformBundle)]
pub struct EcsModule;
