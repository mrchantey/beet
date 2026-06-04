//! Scoped entity traversal within render-root boundaries.
//!
//! This module provides [`RouteQuery`], a system parameter for iterating
//! entities within the scope of a render root. Many systems, ie markdown
//! rendering, must only operate on entities belonging to a specific render
//! root.
//!
//! The boundary is the *rendered content* entity — the one tagged
//! [`RenderRootOf`] and walked by the `NodeRenderer` — not the [`RenderRoot`]
//! handle that names it. The two coincide for self-referential roots, but an
//! ephemeral coordinator route points its handle at separate content, and only
//! the content lives in the traversed tree.
//!
//! # Traversal Rules
//!
//! Given an entity, RouteQuery:
//! 1. Traverses up to find the containing [`RenderRootOf`], or root if none exists
//! 2. Iterates descendants within that render root, stopping at and excluding
//!    nested [`RenderRootOf`] boundaries (unless it's the render root itself)

use crate::prelude::*;
use beet_core::prelude::*;
use std::collections::VecDeque;

/// System parameter for traversing entities within render-root boundaries.
///
/// Provides DFS and BFS iterators that respect render-root boundaries,
/// ensuring systems only process entities belonging to a specific render root.
#[derive(SystemParam)]
pub struct RouteQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	children: Query<'w, 's, &'static Children>,
	route_trees: Query<'w, 's, &'static RouteTree>,
	render_roots: Query<'w, 's, (), With<RenderRootOf>>,
}

impl<'w, 's> RouteQuery<'w, 's> {
	/// Finds the route tree for the given entity, if it exists.
	/// This is done by first traversing to the root ancestor,
	/// which is where route trees should exist.
	pub fn route_tree(&self, entity: Entity) -> Result<&RouteTree> {
		let root = self.ancestors.root_ancestor(entity);
		Ok(self.route_trees.get(root)?)
	}

	/// Finds the render root for the given entity.
	///
	/// Traverses [`ChildOf`] ancestors to find the nearest rendered-content root
	/// ([`RenderRootOf`]), or returns the root ancestor if none is found. The
	/// `ChildOf` walk is acyclic — descendants are not tagged with `RenderRootOf`
	/// — so this is loop-safe.
	pub fn render_root(&self, entity: Entity) -> Entity {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find(|&entity| self.render_roots.contains(entity))
			.unwrap_or_else(|| self.ancestors.root_ancestor(entity))
	}

	/// Returns true if the entity is a rendered-content root ([`RenderRootOf`]),
	/// ie a boundary the traversal stops at.
	pub fn is_render_root(&self, entity: Entity) -> bool {
		self.render_roots.contains(entity)
	}

	/// Creates a depth-first iterator over entities within the render.
	///
	/// Starts from the given entity's render root and traverses descendants,
	/// stopping at [`RenderRoot`] boundaries.
	pub fn iter_dfs(&self, entity: Entity) -> RenderDfsIter<'_, 'w, 's> {
		let root = self.render_root(entity);
		RenderDfsIter {
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
				|entity: In<Entity>, query: RouteQuery| {
					query.iter_dfs(*entity).collect::<Vec<_>>()
				},
				entity,
			)
			.unwrap()
	}

	/// Creates a breadth-first iterator over entities within the render.
	///
	/// Starts from the given entity's render root and traverses descendants,
	/// stopping at [`RenderRoot`] boundaries.
	pub fn iter_bfs(&self, entity: Entity) -> RenderBfsIter<'_, 'w, 's> {
		let root = self.render_root(entity);
		RenderBfsIter {
			query: self,
			queue: VecDeque::from([root]),
			root,
		}
	}

	/// Creates a depth-first iterator starting from a specific root entity.
	///
	/// Unlike [`iter_dfs`](Self::iter_dfs), this does not resolve the render
	/// root first - it uses the provided entity as the root directly.
	pub fn iter_dfs_from(&self, root: Entity) -> RenderDfsIter<'_, 'w, 's> {
		RenderDfsIter {
			query: self,
			stack: vec![root],
			root,
		}
	}

	/// Creates a breadth-first iterator starting from a specific root entity.
	///
	/// Unlike [`iter_bfs`](Self::iter_bfs), this does not resolve the render
	/// root first - it uses the provided entity as the root directly.
	pub fn iter_bfs_from(&self, root: Entity) -> RenderBfsIter<'_, 'w, 's> {
		RenderBfsIter {
			query: self,
			queue: VecDeque::from([root]),
			root,
		}
	}
}

/// Depth-first iterator over entities within a render-route boundary.
pub struct RenderDfsIter<'a, 'w, 's> {
	query: &'a RouteQuery<'w, 's>,
	stack: Vec<Entity>,
	root: Entity,
}

impl Iterator for RenderDfsIter<'_, '_, '_> {
	type Item = Entity;

	fn next(&mut self) -> Option<Self::Item> {
		let entity = self.stack.pop()?;

		// Add children in reverse order for correct DFS traversal
		if let Ok(children) = self.query.children.get(entity) {
			for child in children.iter().rev() {
				// Stop at render-root boundaries, unless the child is the root
				if child != self.root && self.query.is_render_root(child) {
					continue;
				}
				self.stack.push(child);
			}
		}

		Some(entity)
	}
}

/// Breadth-first iterator over entities within a render-route boundary.
pub struct RenderBfsIter<'a, 'w, 's> {
	query: &'a RouteQuery<'w, 's>,
	queue: VecDeque<Entity>,
	root: Entity,
}

impl Iterator for RenderBfsIter<'_, '_, '_> {
	type Item = Entity;

	fn next(&mut self) -> Option<Self::Item> {
		let entity = self.queue.pop_front()?;

		// Add children to queue
		if let Ok(children) = self.query.children.get(entity) {
			for child in children.iter() {
				// Stop at render-root boundaries, unless the child is the root
				if child != self.root && self.query.is_render_root(child) {
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

	/// Spawns a self-referential [`RenderRoot`] boundary entity.
	fn render_root(world: &mut World) -> Entity {
		let entity = world.spawn_empty().id();
		world.entity_mut(entity).insert(RenderRootOf(entity));
		entity
	}

	/// Spawns a self-referential [`RenderRoot`] boundary entity as a child.
	fn render_root_child(world: &mut World, parent: Entity) -> Entity {
		let entity = world.spawn(ChildOf(parent)).id();
		world.entity_mut(entity).insert(RenderRootOf(entity));
		entity
	}

	#[beet_core::test]
	fn render_root_finds_render() {
		let mut world = World::new();

		let render = render_root(&mut world);
		let child = world.spawn(ChildOf(render)).id();
		let grandchild = world.spawn(ChildOf(child)).id();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, query: RouteQuery| {
					query.render_root(entity).xpect_eq(entity);
				},
				render,
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Entity)>,
				 query: RouteQuery| {
					query.render_root(entity).xpect_eq(expected);
				},
				(child, render),
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Entity)>,
				 query: RouteQuery| {
					query.render_root(entity).xpect_eq(expected);
				},
				(grandchild, render),
			)
			.unwrap();
	}

	#[beet_core::test]
	fn render_root_falls_back_to_root() {
		let mut world = World::new();

		let root = world.spawn_empty().id();
		let child = world.spawn(ChildOf(root)).id();

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Entity)>,
				 query: RouteQuery| {
					query.render_root(entity).xpect_eq(expected);
				},
				(child, root),
			)
			.unwrap();
	}

	#[beet_core::test]
	fn dfs_traverses_children() {
		let mut world = World::new();

		// Build: root -> [a, b -> [c, d]]
		let root = render_root(&mut world);
		let child_a = world.spawn(ChildOf(root)).id();
		let child_b = world.spawn(ChildOf(root)).id();
		let grandchild_c = world.spawn(ChildOf(child_b)).id();
		let grandchild_d = world.spawn(ChildOf(child_b)).id();

		let expected = vec![root, child_a, child_b, grandchild_c, grandchild_d];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: RouteQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// DFS order: root, a, b, c, d
					entities.xpect_eq(expected);
				},
				(root, expected),
			)
			.unwrap();
	}

	#[beet_core::test]
	fn bfs_traverses_children() {
		let mut world = World::new();

		// Build: root -> [a, b -> [c, d]]
		let root = render_root(&mut world);
		let child_a = world.spawn(ChildOf(root)).id();
		let child_b = world.spawn(ChildOf(root)).id();
		let grandchild_c = world.spawn(ChildOf(child_b)).id();
		let grandchild_d = world.spawn(ChildOf(child_b)).id();

		let expected = vec![root, child_a, child_b, grandchild_c, grandchild_d];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: RouteQuery| {
					let entities: Vec<_> = query.iter_bfs(entity).collect();
					// BFS order: root, a, b, c, d
					entities.xpect_eq(expected);
				},
				(root, expected),
			)
			.unwrap();
	}

	#[beet_core::test]
	fn stops_at_nested_render() {
		let mut world = World::new();

		let render = render_root(&mut world);
		let child = world.spawn(ChildOf(render)).id();
		let nested_render = render_root_child(&mut world, child);
		let _nested_child = world.spawn(ChildOf(nested_render)).id();

		let expected = vec![render, child];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: RouteQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// Should not include nested_render or its children
					entities.xpect_eq(expected);
				},
				(render, expected),
			)
			.unwrap();
	}

	#[beet_core::test]
	fn stops_at_render_boundary() {
		let mut world = World::new();

		let render = render_root(&mut world);
		let child = world.spawn(ChildOf(render)).id();
		let boundary = render_root_child(&mut world, child);
		let _boundary_child = world.spawn(ChildOf(boundary)).id();

		let expected = vec![render, child];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: RouteQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// Should not include boundary or its children
					entities.xpect_eq(expected);
				},
				(render, expected),
			)
			.unwrap();
	}

	#[beet_core::test]
	fn iter_from_child_finds_render_root() {
		let mut world = World::new();

		let render = render_root(&mut world);
		let child = world.spawn(ChildOf(render)).id();
		let grandchild = world.spawn(ChildOf(child)).id();

		let expected = vec![render, child, grandchild];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: RouteQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// Starting from grandchild should still traverse from root
					entities.xpect_eq(expected);
				},
				(grandchild, expected),
			)
			.unwrap();
	}

	#[beet_core::test]
	fn iter_dfs_from_starts_at_given_entity() {
		let mut world = World::new();

		let render = render_root(&mut world);
		let child = world.spawn(ChildOf(render)).id();
		let grandchild = world.spawn(ChildOf(child)).id();

		let expected = vec![child, grandchild];
		// suppress unused warning
		let _ = render;

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: RouteQuery| {
					let entities: Vec<_> =
						query.iter_dfs_from(entity).collect();
					// Should start from child, not render root
					entities.xpect_eq(expected);
				},
				(child, expected),
			)
			.unwrap();
	}
}
