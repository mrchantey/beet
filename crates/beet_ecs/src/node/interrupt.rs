use crate::prelude::*;
use bevy_ecs::prelude::*;
use std::fmt::Debug;

/// Indicate this node should stop running.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Component, PartialEq)]
#[component(storage = "SparseSet")]
pub struct Interrupt;

pub fn sync_interrupts(
	mut commands: Commands,
	interrupts: Query<Entity, Added<Interrupt>>,
	edges: Query<&Edges>,
) {
	for entity in interrupts.iter() {
		Edges::visit_dfs(entity, &edges, |edge| {
			commands
				.entity(edge)
				.remove::<(Interrupt, Running, RunResult)>();
		});
	}
}
