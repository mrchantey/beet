//! Extension methods for hierarchy queries.

use crate::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::relationship::AncestorIter;
use bevy::ecs::relationship::DescendantDepthFirstIter;
use bevy::ecs::relationship::DescendantIter;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::relationship::SourceIter;
use core::iter::Chain;



/// Extension trait adding hierarchy traversal methods to [`Query`].
#[extend::ext(name=HierarchyQueryExtExt)]
pub impl<
	'w,
	's,
	D: QueryData,
	F: QueryFilter,
	// T: HierarchyQueryExt<'w, 's, D, F>,
> Query<'w, 's, D, F>
{
	/// Iterates over all direct children of the given entity.
	fn iter_direct_descendants<S: RelationshipTarget>(
		&'w self,
		entity: Entity,
	) -> DirectDescendantIter
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
	{
		DirectDescendantIter::new(self, entity)
	}

	/// Iterates over all ancestors of the given entity, including the entity itself.
	fn iter_ancestors_inclusive<R: Relationship>(
		&'w self,
		entity: Entity,
	) -> Chain<core::iter::Once<Entity>, AncestorIter<'w, 's, D, F, R>>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
	{
		Iterator::chain(core::iter::once(entity), self.iter_ancestors(entity))
	}

	/// Iterates over ancestors of the given entity, stopping on the first
	/// revisited entity. Unlike [`iter_ancestors`](Self::iter_ancestors) this
	/// is safe for self-referential relationships (eg those derived with
	/// `allow_self_referential`), which would otherwise loop forever.
	fn iter_ancestors_once<R: Relationship>(
		&'w self,
		entity: Entity,
	) -> AncestorOnceIter<'w, 's, D, F, R>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
	{
		AncestorOnceIter {
			query: self,
			visited: HashSet::default(),
			next: self.get(entity).ok().map(R::get),
			marker: core::marker::PhantomData,
		}
	}

	/// Like [`iter_ancestors_once`](Self::iter_ancestors_once) but yields the
	/// entity itself first. Loop-safe for self-referential relationships.
	fn iter_ancestors_inclusive_once<R: Relationship>(
		&'w self,
		entity: Entity,
	) -> AncestorOnceIter<'w, 's, D, F, R>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
	{
		AncestorOnceIter {
			query: self,
			visited: HashSet::default(),
			next: Some(entity),
			marker: core::marker::PhantomData,
		}
	}

	/// Iterates breadth-first over all descendants of the given entity, including the entity itself.
	fn iter_descendants_inclusive<S: RelationshipTarget>(
		&'w self,
		entity: Entity,
	) -> Chain<core::iter::Once<Entity>, DescendantIter<'w, 's, D, F, S>>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
	{
		Iterator::chain(core::iter::once(entity), self.iter_descendants(entity))
	}

	/// Iterates depth-first over all descendants of the given entity, including the entity itself.
	fn iter_descendants_inclusive_depth_first<S: RelationshipTarget>(
		&'w self,
		entity: Entity,
	) -> Chain<
		core::iter::Once<Entity>,
		DescendantDepthFirstIter<'w, 's, D, F, S>,
	>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
		SourceIter<'w, S>: DoubleEndedIterator,
	{
		Iterator::chain(
			core::iter::once(entity),
			self.iter_descendants_depth_first(entity),
		)
	}

	/// Collects entities in pre-order (root first, depth-first), including the root.
	fn collect_pre_order(&'w self, entity: Entity) -> Vec<Entity>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w Children>,
	{
		let mut result = Vec::new();
		let mut stack = vec![entity];
		while let Some(entity) = stack.pop() {
			result.push(entity);
			if let Ok(children) = self.get(entity) {
				stack.extend(children.iter().rev());
			}
		}
		result
	}

	/// Collects entities in post-order (leaves first, depth-first), including the root.
	fn collect_post_order(&'w self, entity: Entity) -> Vec<Entity>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w Children>,
	{
		let mut result = Vec::new();
		let mut stack = vec![(entity, false)];
		while let Some((entity, visited)) = stack.pop() {
			if visited {
				result.push(entity);
			} else {
				stack.push((entity, true));
				if let Ok(children) = self.get(entity) {
					stack.extend(
						children.iter().rev().map(|child| (child, false)),
					);
				}
			}
		}
		result
	}
}




/// An [`Iterator`] over the direct descendants of an [`Entity`].
///
/// Unlike [`DescendantIter`], this only yields immediate children,
/// not the entire subtree.
pub struct DirectDescendantIter {
	vec: Vec<Entity>,
}

impl DirectDescendantIter {
	/// Creates a new [`DirectDescendantIter`] for the given entity.
	pub fn new<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget>(
		children_query: &'w Query<'w, 's, D, F>,
		entity: Entity,
	) -> Self
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
	{
		let mut items = children_query
			.get(entity)
			.into_iter()
			.flat_map(RelationshipTarget::iter)
			.collect::<Vec<_>>();
		// we pop from the end, so reverse to maintain order
		items.reverse();
		Self { vec: items }
	}
}

impl Iterator for DirectDescendantIter {
	type Item = Entity;
	fn next(&mut self) -> Option<Self::Item> { self.vec.pop() }
}




/// A loop-safe [`Iterator`] over the ancestors of an [`Entity`].
///
/// Tracks visited entities and stops on the first revisit, so it terminates
/// even on self-referential relationships. Created via
/// [`iter_ancestors_once`](HierarchyQueryExtExt::iter_ancestors_once) and
/// [`iter_ancestors_inclusive_once`](HierarchyQueryExtExt::iter_ancestors_inclusive_once).
pub struct AncestorOnceIter<'w, 's, D: QueryData, F: QueryFilter, R: Relationship>
{
	query: &'w Query<'w, 's, D, F>,
	visited: HashSet<Entity>,
	next: Option<Entity>,
	marker: core::marker::PhantomData<R>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, R: Relationship> Iterator
	for AncestorOnceIter<'w, 's, D, F, R>
where
	D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
	type Item = Entity;
	fn next(&mut self) -> Option<Entity> {
		let current = self.next.take()?;
		// stop the moment we'd revisit an entity, breaking any cycle
		if !self.visited.insert(current) {
			return None;
		}
		self.next = self.query.get(current).ok().map(R::get);
		Some(current)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(Component)]
	#[relationship(relationship_target = SelfTargets, allow_self_referential)]
	struct SelfRel(Entity);
	#[derive(Component)]
	#[relationship_target(relationship = SelfRel)]
	struct SelfTargets(Vec<Entity>);

	#[beet_core::test]
	fn iter_ancestors_once_terminates_on_self_reference() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		world.entity_mut(entity).insert(SelfRel(entity));

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, query: Query<&SelfRel>| {
					// exclusive variant still yields the (self) ancestor once
					query
						.iter_ancestors_once(entity)
						.collect::<Vec<_>>()
						.xpect_eq(vec![entity]);
					// inclusive variant yields only the entity, then stops
					query
						.iter_ancestors_inclusive_once(entity)
						.collect::<Vec<_>>()
						.xpect_eq(vec![entity]);
				},
				entity,
			)
			.unwrap();
	}

	#[beet_core::test]
	fn iter_ancestors_once_walks_normal_chain() {
		let mut world = World::new();
		let a = world.spawn_empty().id();
		let b = world.spawn(ChildOf(a)).id();
		let c = world.spawn(ChildOf(b)).id();

		world
			.run_system_cached_with(
				|In((a, b, c)): In<(Entity, Entity, Entity)>,
				 query: Query<&ChildOf>| {
					query
						.iter_ancestors_once(c)
						.collect::<Vec<_>>()
						.xpect_eq(vec![b, a]);
					query
						.iter_ancestors_inclusive_once(c)
						.collect::<Vec<_>>()
						.xpect_eq(vec![c, b, a]);
				},
				(a, b, c),
			)
			.unwrap();
	}
}
