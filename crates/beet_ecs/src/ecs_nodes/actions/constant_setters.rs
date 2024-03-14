use crate::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;


// #[action(
// 	system=constant_score,
// 	set=PreTickSet,
// 	components=Score::default()
// )]
// #[reflect(Component, Action)]
#[derive(Default, PartialEq, Deref, DerefMut)]
#[derive_action(set=PreTickSet)]
pub struct SetOnStart<T: Clone + Component>(pub T);

impl<T: Clone + Component> SetOnStart<T> {
	pub fn new(value: T) -> Self { Self(value) }
}

fn set_on_start<T: Clone + Component>(
	mut query: Query<(&SetOnStart<T>, &mut T), Added<SetOnStart<T>>>,
) {
	for (from, mut to) in query.iter_mut() {
		*to = from.0.clone();
	}
}


#[derive(Default, PartialEq, Deref, DerefMut)]
#[derive_action(set=PreTickSet)]
pub struct InsertOnRun<T: Clone + Component>(pub T);

impl<T: Clone + Component> InsertOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

// this was SetRunResult - With<Running> check for regression
fn insert_on_run<T: Clone + Component>(
	mut commands: Commands,
	query: Query<(Entity, &InsertOnRun<T>), Added<Running>>,
) {
	for (entity, from) in query.iter() {
		commands.entity(entity).insert(from.0.clone());
	}
}

/// If the node does not have the component this will do nothing.
#[derive(Default, PartialEq, Deref, DerefMut)]
#[derive_action(set=PreTickSet)]
pub struct SetOnRun<T: Clone + Component>(pub T);

impl<T: Clone + Component> SetOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_on_run<T: Clone + Component>(
	mut query: Query<(&SetOnRun<T>, &mut T), Added<Running>>,
) {
	for (from, mut to) in query.iter_mut() {
		*to = from.0.clone();
	}
}

/// If the agent does not have the component this will do nothing.
#[derive(Default, PartialEq, Deref, DerefMut)]
#[derive_action(set=PreTickSet)]
pub struct SetAgentOnRun<T: Clone + Component>(pub T);

impl<T: Clone + Component> SetAgentOnRun<T> {
	pub fn new(value: T) -> Self { Self(value.into()) }
}

fn set_agent_on_run<T: Clone + Component>(
	mut agents: Query<&mut T>,
	mut query: Query<(&TargetAgent, &SetAgentOnRun<T>), Added<Running>>,
) {
	for (entity, src) in query.iter_mut() {
		if let Ok(mut dst) = agents.get_mut(**entity) {
			*dst = src.0.clone();
		}
	}
}
