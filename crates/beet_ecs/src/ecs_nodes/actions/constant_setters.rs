use crate::prelude::*;
use bevy::prelude::*;

// #[action(
// 	system=constant_score,
// 	set=PreTickSet,
// 	components=Score::default()
// )]
// #[reflect(Component, Action)]
#[derive(PartialEq, Deref, DerefMut)]
#[derive_action]
#[action(set=PreTickSet)]
#[action(graph_role=GraphRole::Node)]
pub struct SetOnStart<T: Default + Clone + Component>(pub T);

impl<T: Default + Clone + Component> SetOnStart<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_on_start<T: Default + Clone + Component>(
	mut query: Query<(&SetOnStart<T>, &mut T), Added<SetOnStart<T>>>,
) {
	for (from, mut to) in query.iter_mut() {
		*to = from.0.clone();
	}
}

#[derive_action]
#[derive(PartialEq, Deref, DerefMut)]
#[action(set=PreTickSet)]
#[action(graph_role=GraphRole::Node)]
pub struct InsertOnRun<T: Default + Clone + Component>(pub T);

impl<T: Default + Clone + Component> InsertOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

// this was SetRunResult - With<Running> check for regression
fn insert_on_run<T: Default + Clone + Component>(
	mut commands: Commands,
	query: Query<(Entity, &InsertOnRun<T>), Added<Running>>,
) {
	for (entity, from) in query.iter() {
		commands.entity(entity).insert(from.0.clone());
	}
}

/// If the node does not have the component this will do nothing.
#[derive(PartialEq, Deref, DerefMut)]
#[derive_action]
#[action(set=PostTickSet)]
#[action(graph_role=GraphRole::Node)]
pub struct SetOnRun<T: Default + Clone + Component>(pub T);

impl<T: Default + Clone + Component> SetOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_on_run<T: Default + Clone + Component>(
	mut query: Query<(&SetOnRun<T>, &mut T), Added<Running>>,
) {
	for (from, mut to) in query.iter_mut() {
		*to = from.0.clone();
	}
}

/// If the agent does not have the component this will do nothing.
#[derive(PartialEq, Deref, DerefMut)]
#[derive_action]
#[action(set=PostTickSet)]
#[action(graph_role=GraphRole::Agent)]
pub struct SetAgentOnRun<T: Default + Clone + Component>(pub T);

impl<T: Default + Clone + Component> SetAgentOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_agent_on_run<T: Default + Clone + Component>(
	mut agents: Query<&mut T>,
	mut query: Query<(&TargetAgent, &SetAgentOnRun<T>), Added<Running>>,
) {
	for (entity, src) in query.iter_mut() {
		if let Ok(mut dst) = agents.get_mut(**entity) {
			*dst = src.0.clone();
		}
	}
}
