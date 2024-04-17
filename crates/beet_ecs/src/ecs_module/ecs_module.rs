use crate::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, BeetModule)]
#[actions(
	// constants
	SetOnSpawn::<Score>,
	InsertOnRun::<RunResult>,
	// utility
	EmptyAction,
	Repeat,
	InsertInDuration::<RunResult>,
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
	RootIsTargetAgent,
	TargetAgent,
	ActionTarget,
	// bevy
	Name,
	// transform bundle
	Transform,
	GlobalTransform,
)]
#[bundles(TransformBundle)]
/// The core actions and components required for most behaviors.
pub struct EcsModule;
