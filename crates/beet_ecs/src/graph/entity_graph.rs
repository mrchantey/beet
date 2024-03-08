use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_utils::HashMap;
use petgraph::graph::DiGraph;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Component)]
pub struct EntityGraph(pub DiGraph<Entity, ()>);


impl EntityGraph {
	pub fn despawn(&self, commands: &mut Commands) {
		for entity in self.node_weights() {
			commands.entity(*entity).despawn();
		}
	}
}

#[derive(Debug, Default, Clone, Deref, DerefMut, Resource)]
pub struct TrackedEntityGraphs(pub HashMap<Entity, EntityGraph>);


/// This mechanism requires at least one frame between spawning a graph
/// and deleting it
pub fn cleanup_entity_graph(
	mut commands: Commands,
	mut tracked_entities: ResMut<TrackedEntityGraphs>,
	added_graphs: Query<(Entity, &EntityGraph), Changed<EntityGraph>>,
	mut entity_graphs: RemovedComponents<EntityGraph>,
) {

	for (entity, graph) in added_graphs.iter() {
		tracked_entities.insert(entity, graph.clone());
	}

	for entity in entity_graphs.read() {
		if let Some(graph) = tracked_entities.remove(&entity) {
			graph.despawn(&mut commands);
		} else {
			log::warn!("Entity {entity:?} not found in tracked entity graphs");
		}
	}
}
