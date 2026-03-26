//! Scoped entity traversal within scene route boundaries.
//!
//! This module provides [`SceneRouteQuery`], a system parameter for iterating
//! entities within the scope of a [`SceneRoute`]. Many systems, ie markdown
//! rendering, must only operate on entities belonging to a specific scene.
//!
//! # Traversal Rules
//!
//! Given an entity, SceneRouteQuery:
//! 1. Traverses up to find the containing [`SceneRoute`], or root if no scene exists
//! 2. Iterates descendants within that scene, stopping at and excluding
//! [`SceneRoute`] boundaries (unless it's the scene root itself)

use crate::prelude::*;
use beet_core::prelude::*;
use std::collections::VecDeque;

/// System parameter for traversing entities within scene route boundaries.
///
/// Provides DFS and BFS iterators that respect scene route boundaries,
/// ensuring systems only process entities belonging to a specific scene.
#[derive(SystemParam)]
pub struct SceneRouteQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	children: Query<'w, 's, &'static Children>,
	route_trees: Query<'w, 's, &'static RouteTree>,
	scenes: Query<'w, 's, (), With<SceneRoute>>,
}

impl<'w, 's> SceneRouteQuery<'w, 's> {
	/// Finds the route tree for the given entity, if it exists.
	/// This is done by first traversing to the root ancestor,
	/// which is where route trees should exist.
	pub fn route_tree(&self, entity: Entity) -> Result<&RouteTree> {
		let root = self.ancestors.root_ancestor(entity);
		Ok(self.route_trees.get(root)?)
	}

	/// Finds the scene root for the given entity.
	///
	/// Traverses ancestors to find the nearest [`SceneRoute`], or returns
	/// the root ancestor if no scene is found.
	pub fn scene_root(&self, entity: Entity) -> Entity {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find(|&entity| self.scenes.contains(entity))
			.unwrap_or_else(|| self.ancestors.root_ancestor(entity))
	}

	/// Returns true if the entity is a [`SceneRoute`].
	pub fn is_scene(&self, entity: Entity) -> bool {
		self.scenes.contains(entity)
	}

	/// Returns true if the entity is a SceneRoute boundary.
	fn is_boundary(&self, entity: Entity) -> bool { self.is_scene(entity) }

	/// Creates a depth-first iterator over entities within the scene.
	///
	/// Starts from the given entity's scene root and traverses descendants,
	/// stopping at SceneRoute boundaries.
	pub fn iter_dfs(&self, entity: Entity) -> SceneRouteDfsIter<'_, 'w, 's> {
		let root = self.scene_root(entity);
		SceneRouteDfsIter {
			query: self,
			stack: vec![root],
			root,
		}
	}

	/// Collects DFS entities for the given entity using exclusive world access.
	pub fn iter_dfs_exclusive(
		world: &mut World,
		entity: Entity,
	) -> Vec<Entity> {
		world
			.run_system_once_with(
				|entity: In<Entity>, query: SceneRouteQuery| {
					query.iter_dfs(*entity).collect::<Vec<_>>()
				},
				entity,
			)
			.unwrap()
	}

	/// Creates a breadth-first iterator over entities within the scene.
	///
	/// Starts from the given entity's scene root and traverses descendants,
	/// stopping at SceneRoute boundaries.
	pub fn iter_bfs(&self, entity: Entity) -> SceneRouteBfsIter<'_, 'w, 's> {
		let root = self.scene_root(entity);
		SceneRouteBfsIter {
			query: self,
			queue: VecDeque::from([root]),
			root,
		}
	}

	/// Creates a depth-first iterator starting from a specific root entity.
	///
	/// Unlike [`iter_dfs`](Self::iter_dfs), this does not resolve the scene root
	/// first - it uses the provided entity as the root directly.
	pub fn iter_dfs_from(&self, root: Entity) -> SceneRouteDfsIter<'_, 'w, 's> {
		SceneRouteDfsIter {
			query: self,
			stack: vec![root],
			root,
		}
	}

	/// Creates a breadth-first iterator starting from a specific root entity.
	///
	/// Unlike [`iter_bfs`](Self::iter_bfs), this does not resolve the scene root
	/// first - it uses the provided entity as the root directly.
	pub fn iter_bfs_from(&self, root: Entity) -> SceneRouteBfsIter<'_, 'w, 's> {
		SceneRouteBfsIter {
			query: self,
			queue: VecDeque::from([root]),
			root,
		}
	}
}

/// Depth-first iterator over entities within a scene route boundary.
pub struct SceneRouteDfsIter<'a, 'w, 's> {
	query: &'a SceneRouteQuery<'w, 's>,
	stack: Vec<Entity>,
	root: Entity,
}

impl Iterator for SceneRouteDfsIter<'_, '_, '_> {
	type Item = Entity;

	fn next(&mut self) -> Option<Self::Item> {
		let entity = self.stack.pop()?;

		// Add children in reverse order for correct DFS traversal
		if let Ok(children) = self.query.children.get(entity) {
			for child in children.iter().rev() {
				// Stop at boundaries (SceneRoute), unless the child is the root itself
				if child != self.root && self.query.is_boundary(child) {
					continue;
				}
				self.stack.push(child);
			}
		}

		Some(entity)
	}
}

/// Breadth-first iterator over entities within a scene route boundary.
pub struct SceneRouteBfsIter<'a, 'w, 's> {
	query: &'a SceneRouteQuery<'w, 's>,
	queue: VecDeque<Entity>,
	root: Entity,
}

impl Iterator for SceneRouteBfsIter<'_, '_, '_> {
	type Item = Entity;

	fn next(&mut self) -> Option<Self::Item> {
		let entity = self.queue.pop_front()?;

		// Add children to queue
		if let Ok(children) = self.query.children.get(entity) {
			for child in children.iter() {
				// Stop at boundaries (SceneRoute), unless the child is the root itself
				if child != self.root && self.query.is_boundary(child) {
					continue;
				}
				self.queue.push_back(child);
			}
		}

		Some(entity)
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn scene_root_finds_scene() {
		let mut world = World::new();

		let scene = world.spawn(SceneRoute).id();
		let child = world.spawn(ChildOf(scene)).id();
		let grandchild = world.spawn(ChildOf(child)).id();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, query: SceneRouteQuery| {
					query.scene_root(entity).xpect_eq(entity);
				},
				scene,
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Entity)>,
				 query: SceneRouteQuery| {
					query.scene_root(entity).xpect_eq(expected);
				},
				(child, scene),
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Entity)>,
				 query: SceneRouteQuery| {
					query.scene_root(entity).xpect_eq(expected);
				},
				(grandchild, scene),
			)
			.unwrap();
	}

	#[test]
	fn scene_root_falls_back_to_root() {
		let mut world = World::new();

		let root = world.spawn_empty().id();
		let child = world.spawn(ChildOf(root)).id();

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Entity)>,
				 query: SceneRouteQuery| {
					query.scene_root(entity).xpect_eq(expected);
				},
				(child, root),
			)
			.unwrap();
	}

	#[test]
	fn dfs_traverses_children() {
		let mut world = World::new();

		// Build: root -> [a, b -> [c, d]]
		let root = world.spawn(SceneRoute).id();
		let child_a = world.spawn(ChildOf(root)).id();
		let child_b = world.spawn(ChildOf(root)).id();
		let grandchild_c = world.spawn(ChildOf(child_b)).id();
		let grandchild_d = world.spawn(ChildOf(child_b)).id();

		let expected = vec![root, child_a, child_b, grandchild_c, grandchild_d];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: SceneRouteQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// DFS order: root, a, b, c, d
					entities.xpect_eq(expected);
				},
				(root, expected),
			)
			.unwrap();
	}

	#[test]
	fn bfs_traverses_children() {
		let mut world = World::new();

		// Build: root -> [a, b -> [c, d]]
		let root = world.spawn(SceneRoute).id();
		let child_a = world.spawn(ChildOf(root)).id();
		let child_b = world.spawn(ChildOf(root)).id();
		let grandchild_c = world.spawn(ChildOf(child_b)).id();
		let grandchild_d = world.spawn(ChildOf(child_b)).id();

		let expected = vec![root, child_a, child_b, grandchild_c, grandchild_d];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: SceneRouteQuery| {
					let entities: Vec<_> = query.iter_bfs(entity).collect();
					// BFS order: root, a, b, c, d
					entities.xpect_eq(expected);
				},
				(root, expected),
			)
			.unwrap();
	}

	#[test]
	fn stops_at_nested_scene() {
		let mut world = World::new();

		let scene = world.spawn(SceneRoute).id();
		let child = world.spawn(ChildOf(scene)).id();
		let nested_scene = world.spawn((SceneRoute, ChildOf(child))).id();
		let _nested_child = world.spawn(ChildOf(nested_scene)).id();

		let expected = vec![scene, child];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: SceneRouteQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// Should not include nested_scene or its children
					entities.xpect_eq(expected);
				},
				(scene, expected),
			)
			.unwrap();
	}

	#[test]
	fn stops_at_scene_boundary() {
		let mut world = World::new();

		let scene = world.spawn(SceneRoute).id();
		let child = world.spawn(ChildOf(scene)).id();
		let boundary = world.spawn((SceneRoute, ChildOf(child))).id();
		let _boundary_child = world.spawn(ChildOf(boundary)).id();

		let expected = vec![scene, child];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: SceneRouteQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// Should not include boundary or its children
					entities.xpect_eq(expected);
				},
				(scene, expected),
			)
			.unwrap();
	}

	#[test]
	fn iter_from_child_finds_scene_root() {
		let mut world = World::new();

		let scene = world.spawn(SceneRoute).id();
		let child = world.spawn(ChildOf(scene)).id();
		let grandchild = world.spawn(ChildOf(child)).id();

		let expected = vec![scene, child, grandchild];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: SceneRouteQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// Starting from grandchild should still traverse from scene root
					entities.xpect_eq(expected);
				},
				(grandchild, expected),
			)
			.unwrap();
	}

	#[test]
	fn iter_dfs_from_starts_at_given_entity() {
		let mut world = World::new();

		let scene = world.spawn(SceneRoute).id();
		let child = world.spawn(ChildOf(scene)).id();
		let grandchild = world.spawn(ChildOf(child)).id();

		let expected = vec![child, grandchild];
		// suppress unused warning
		let _ = scene;

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: SceneRouteQuery| {
					let entities: Vec<_> =
						query.iter_dfs_from(entity).collect();
					// Should start from child, not scene root
					entities.xpect_eq(expected);
				},
				(child, expected),
			)
			.unwrap();
	}
}
