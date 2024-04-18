use crate::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, BeetModule)]
#[actions(
	// lifecycle
	InsertInDuration::<RunResult>,
	InsertOnRun::<RunResult>,
	LogNameOnRun,
	LogOnRun,
	Repeat,
	SetOnSpawn::<Score>,
	// selectors
	SequenceSelector,
	FallbackSelector,
	ScoreSelector,
	// utility
	EmptyAction,
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
