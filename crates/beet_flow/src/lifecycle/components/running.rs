use bevy::prelude::*;
use std::fmt::Debug;

/// Indicate this node is currently running.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Copy, Clone, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default)]
pub struct Running;




/// Indicate the result of an action.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Default)]
pub enum RunResult {
	#[default]
	/// The Action was successful.
	Success,
	/// The Action was unsuccessful.
	Failure,
}