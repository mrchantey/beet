use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use petgraph::graph::DiGraph;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deref, DerefMut, Serialize, Deserialize)]
pub struct EntityGraph(pub DiGraph<Entity, ()>);


impl EntityGraph {
	pub fn despawn(&self, commands: &mut Commands) {
		for entity in self.node_weights() {
			commands.entity(*entity).despawn();
		}
	}
}
