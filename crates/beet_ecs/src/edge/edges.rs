use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use bevy::utils::HashSet;

/// Outgoing edges of an action, aka Children.
#[derive(Debug, Default, Clone, Deref, DerefMut, Component, Reflect)]
#[reflect(Component, MapEntities)]
pub struct Edges(pub Vec<Entity>);
impl MapEntities for Edges {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		for entity in &mut self.0 {
			*entity = entity_mapper.map_entity(*entity);
		}
	}
}

impl Edges {
	pub fn new() -> Self { Self(Vec::new()) }

	pub fn with_child(mut self, edge: Entity) -> Self {
		self.push(edge);
		self
	}

	pub fn despawn_recursive(
		commands: &mut Commands,
		entity: Entity,
		edges: &Query<&Edges>,
	) {
		Edges::visit_dfs(entity, &edges, |entity| {
			commands.entity(entity).despawn();
		});
	}


	pub fn visit_dfs(
		entity: Entity,
		edge_query: &Query<&Edges>,
		mut func: impl FnMut(Entity),
	) {
		Self::visit_dfs_recursive(
			entity,
			edge_query,
			&mut func,
			&mut HashSet::default(),
		);
	}

	fn visit_dfs_recursive(
		entity: Entity,
		edge_query: &Query<&Edges>,
		func: &mut impl FnMut(Entity),
		visited: &mut HashSet<Entity>,
	) {
		if visited.contains(&entity) {
			return;
		}
		visited.insert(entity);
		func(entity);
		if let Ok(edges) = edge_query.get(entity) {
			for edge in edges.iter() {
				Self::visit_dfs_recursive(*edge, edge_query, func, visited);
			}
		}
	}
}
