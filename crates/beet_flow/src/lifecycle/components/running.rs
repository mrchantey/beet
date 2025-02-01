use bevy::prelude::*;
use std::fmt::Debug;

/// Indicate this node is currently long-running.
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
impl std::fmt::Display for RunResult {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			RunResult::Success => write!(f, "Success"),
			RunResult::Failure => write!(f, "Failure"),
		}
	}
}
