use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use strum_macros::Display;
use strum_macros::EnumIter;


/// Indicate this node is currently running.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Copy, Clone, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
pub struct Running;




/// Indicate the result of an action.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(
	Default,
	Debug,
	Clone,
	Copy,
	Component,
	PartialEq,
	Serialize,
	Deserialize,
	EnumIter,
	Display,
	Reflect,
)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
pub enum RunResult {
	#[default]
	/// The Action was successful.
	Success,
	/// The Action was unsuccessful.
	Failure,
}


/// Syncs [`Running`] and [`RunResult`] components, by default added to [`PostNodeUpdateSet`].
pub fn sync_running(
	mut commands: Commands,
	// occurs immediately after `RunResult` is added
	first_pass: Query<Entity, (Added<RunResult>, With<Running>)>,
	// occurs one frame later
	second_pass: Query<Entity, (With<RunResult>, Without<Running>)>,
) {
	for entity in first_pass.iter() {
		commands.entity(entity).remove::<Running>();
	}
	for entity in second_pass.iter() {
		commands.entity(entity).remove::<RunResult>();
	}
}
