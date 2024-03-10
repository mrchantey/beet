#[allow(unused)]
use crate::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;

/// Added to entites that have at least one associated [`BehaviorGraph`].
/// Remove this component to dispose of all of this agents graphs.
/// This is useful, for example for [`cleanup_entity_graph`] to only listen for removals
/// of agent entities
#[derive(Debug, Copy, Clone, Component)]
pub struct AgentMarker;

/// Added to [`BehaviorNode`] entities that have a target agent.
#[derive(Debug, PartialEq, Deref, DerefMut, Component)]
pub struct TargetAgent(pub Entity);


/// Used by actions to specify some target, ie seek.
#[derive(Debug, PartialEq, Deref, DerefMut, Component)]
pub struct ActionTarget(pub Entity);
