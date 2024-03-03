use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_utils::HashSet;


/// Outgoing edges of an action, aka Children.
#[derive(Debug, Default, Clone, Deref, DerefMut, Component)]
pub struct Edges(pub Vec<Entity>);


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
