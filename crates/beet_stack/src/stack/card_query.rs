//! Scoped entity traversal within card boundaries.
//!
//! This module provides [`CardQuery`], a system parameter for iterating
//! entities within the scope of a [`Card`]. Many systems, ie markdown
//! rendering, must only operate on entities belonging to a specific card.
//!
//! # Traversal Rules
//!
//! Given an entity, CardQuery:
//! 1. Traverses up to find the containing [`Card`], or root if no card exists
//! 2. Iterates descendants within that card, stopping at and excluding
//! [`Card`] boundaries (unless it's the card root itself)
//!
//! # Example
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! fn my_system(card_query: CardQuery) {
//!     // Iterate all text content within entity's card
//!     for text_entity in card_query.iter_dfs(entity) {
//!         // Process entity...
//!     }
//! }
//! ```

use crate::prelude::*;
use beet_core::prelude::*;
use std::collections::VecDeque;

/// System parameter for traversing entities within card boundaries.
///
/// Provides DFS and BFS iterators that respect card/stack boundaries,
/// ensuring systems only process entities belonging to a specific card.
#[derive(SystemParam)]
pub struct CardQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	children: Query<'w, 's, &'static Children>,
	cards: Query<'w, 's, (), With<Card>>,
}

impl<'w, 's> CardQuery<'w, 's> {
	/// Finds the card root for the given entity.
	///
	/// Traverses ancestors to find the nearest [`Card`], or returns
	/// the root ancestor if no card is found.
	pub fn card_root(&self, entity: Entity) -> Entity {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find(|&entity| self.cards.contains(entity))
			.unwrap_or_else(|| self.ancestors.root_ancestor(entity))
	}

	/// Returns true if the entity is a Card boundary.
	fn is_boundary(&self, entity: Entity) -> bool {
		self.cards.contains(entity)
	}

	/// Creates a depth-first iterator over entities within the card.
	///
	/// Starts from the given entity's card root and traverses descendants,
	/// stopping at Card or Stack boundaries.
	pub fn iter_dfs(&self, entity: Entity) -> CardDfsIter<'_, 'w, 's> {
		let root = self.card_root(entity);
		CardDfsIter {
			query: self,
			stack: vec![root],
			root,
		}
	}

	/// Creates a breadth-first iterator over entities within the card.
	///
	/// Starts from the given entity's card root and traverses descendants,
	/// stopping at Card or Stack boundaries.
	pub fn iter_bfs(&self, entity: Entity) -> CardBfsIter<'_, 'w, 's> {
		let root = self.card_root(entity);
		CardBfsIter {
			query: self,
			queue: VecDeque::from([root]),
			root,
		}
	}

	/// Creates a depth-first iterator starting from a specific root entity.
	///
	/// Unlike [`iter_dfs`](Self::iter_dfs), this does not resolve the card root
	/// first - it uses the provided entity as the root directly.
	pub fn iter_dfs_from(&self, root: Entity) -> CardDfsIter<'_, 'w, 's> {
		CardDfsIter {
			query: self,
			stack: vec![root],
			root,
		}
	}

	/// Creates a breadth-first iterator starting from a specific root entity.
	///
	/// Unlike [`iter_bfs`](Self::iter_bfs), this does not resolve the card root
	/// first - it uses the provided entity as the root directly.
	pub fn iter_bfs_from(&self, root: Entity) -> CardBfsIter<'_, 'w, 's> {
		CardBfsIter {
			query: self,
			queue: VecDeque::from([root]),
			root,
		}
	}
}

/// Depth-first iterator over entities within a card boundary.
pub struct CardDfsIter<'a, 'w, 's> {
	query: &'a CardQuery<'w, 's>,
	stack: Vec<Entity>,
	root: Entity,
}

impl Iterator for CardDfsIter<'_, '_, '_> {
	type Item = Entity;

	fn next(&mut self) -> Option<Self::Item> {
		let entity = self.stack.pop()?;

		// Add children in reverse order for correct DFS traversal
		if let Ok(children) = self.query.children.get(entity) {
			for child in children.iter().rev() {
				// Stop at boundaries (Card or Stack), unless the child is the root itself
				if child != self.root && self.query.is_boundary(child) {
					continue;
				}
				self.stack.push(child);
			}
		}

		Some(entity)
	}
}

/// Breadth-first iterator over entities within a card boundary.
pub struct CardBfsIter<'a, 'w, 's> {
	query: &'a CardQuery<'w, 's>,
	queue: VecDeque<Entity>,
	root: Entity,
}

impl Iterator for CardBfsIter<'_, '_, '_> {
	type Item = Entity;

	fn next(&mut self) -> Option<Self::Item> {
		let entity = self.queue.pop_front()?;

		// Add children to queue
		if let Ok(children) = self.query.children.get(entity) {
			for child in children.iter() {
				// Stop at boundaries (Card or Stack), unless the child is the root itself
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
	fn card_root_finds_card() {
		let mut world = World::new();

		let card = world.spawn(Card).id();
		let child = world.spawn(ChildOf(card)).id();
		let grandchild = world.spawn(ChildOf(child)).id();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, query: CardQuery| {
					query.card_root(entity).xpect_eq(entity);
				},
				card,
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Entity)>,
				 query: CardQuery| {
					query.card_root(entity).xpect_eq(expected);
				},
				(child, card),
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Entity)>,
				 query: CardQuery| {
					query.card_root(entity).xpect_eq(expected);
				},
				(grandchild, card),
			)
			.unwrap();
	}

	#[test]
	fn card_root_falls_back_to_root() {
		let mut world = World::new();

		let root = world.spawn_empty().id();
		let child = world.spawn(ChildOf(root)).id();

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Entity)>,
				 query: CardQuery| {
					query.card_root(entity).xpect_eq(expected);
				},
				(child, root),
			)
			.unwrap();
	}

	#[test]
	fn dfs_traverses_children() {
		let mut world = World::new();

		// Build: root -> [a, b -> [c, d]]
		let root = world.spawn(Card).id();
		let child_a = world.spawn(ChildOf(root)).id();
		let child_b = world.spawn(ChildOf(root)).id();
		let grandchild_c = world.spawn(ChildOf(child_b)).id();
		let grandchild_d = world.spawn(ChildOf(child_b)).id();

		let expected = vec![root, child_a, child_b, grandchild_c, grandchild_d];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: CardQuery| {
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
		let root = world.spawn(Card).id();
		let child_a = world.spawn(ChildOf(root)).id();
		let child_b = world.spawn(ChildOf(root)).id();
		let grandchild_c = world.spawn(ChildOf(child_b)).id();
		let grandchild_d = world.spawn(ChildOf(child_b)).id();

		let expected = vec![root, child_a, child_b, grandchild_c, grandchild_d];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: CardQuery| {
					let entities: Vec<_> = query.iter_bfs(entity).collect();
					// BFS order: root, a, b, c, d
					entities.xpect_eq(expected);
				},
				(root, expected),
			)
			.unwrap();
	}

	#[test]
	fn stops_at_nested_card() {
		let mut world = World::new();

		let card = world.spawn(Card).id();
		let child = world.spawn(ChildOf(card)).id();
		let nested_card = world.spawn((Card, ChildOf(child))).id();
		let _nested_child = world.spawn(ChildOf(nested_card)).id();

		let expected = vec![card, child];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: CardQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// Should not include nested_card or its children
					entities.xpect_eq(expected);
				},
				(card, expected),
			)
			.unwrap();
	}

	#[test]
	fn stops_at_stack() {
		let mut world = World::new();

		let card = world.spawn(Card).id();
		let child = world.spawn(ChildOf(card)).id();
		let stack = world.spawn((Card, ChildOf(child))).id();
		let _stack_child = world.spawn(ChildOf(stack)).id();

		let expected = vec![card, child];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: CardQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// Should not include stack or its children
					entities.xpect_eq(expected);
				},
				(card, expected),
			)
			.unwrap();
	}

	#[test]
	fn iter_from_child_finds_card_root() {
		let mut world = World::new();

		let card = world.spawn(Card).id();
		let child = world.spawn(ChildOf(card)).id();
		let grandchild = world.spawn(ChildOf(child)).id();

		let expected = vec![card, child, grandchild];

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: CardQuery| {
					let entities: Vec<_> = query.iter_dfs(entity).collect();
					// Starting from grandchild should still traverse from card root
					entities.xpect_eq(expected);
				},
				(grandchild, expected),
			)
			.unwrap();
	}

	#[test]
	fn iter_dfs_from_starts_at_given_entity() {
		let mut world = World::new();

		let card = world.spawn(Card).id();
		let child = world.spawn(ChildOf(card)).id();
		let grandchild = world.spawn(ChildOf(child)).id();

		let expected = vec![child, grandchild];
		// suppress unused warning
		let _ = card;

		world
			.run_system_cached_with(
				|In((entity, expected)): In<(Entity, Vec<Entity>)>,
				 query: CardQuery| {
					let entities: Vec<_> =
						query.iter_dfs_from(entity).collect();
					// Should start from child, not card root
					entities.xpect_eq(expected);
				},
				(child, expected),
			)
			.unwrap();
	}
}
