use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::relationship::AncestorIter;
use bevy::ecs::relationship::DescendantDepthFirstIter;
use bevy::ecs::relationship::DescendantIter;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::relationship::SourceIter;
use std::iter::Chain;
use crate::prelude::*;



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
	) -> Chain<std::iter::Once<Entity>, AncestorIter<'w, 's, D, F, R>>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
	{
		Iterator::chain(std::iter::once(entity), self.iter_ancestors(entity))
	}

	/// Iterates over all descendants of the given entity, including the entity itself.
	fn iter_descendants_inclusive<S: RelationshipTarget>(
		&'w self,
		entity: Entity,
	) -> Chain<std::iter::Once<Entity>, DescendantIter<'w, 's, D, F, S>>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
	{
		Iterator::chain(std::iter::once(entity), self.iter_descendants(entity))
	}

	/// Iterates depth first over all descendants of the given entity, including the entity itself.
	fn iter_descendants_inclusive_depth_first<S: RelationshipTarget>(
		&'w self,
		entity: Entity,
	) -> Chain<std::iter::Once<Entity>, DescendantDepthFirstIter<'w, 's, D, F, S>>
	where
		D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
		SourceIter<'w, S>: DoubleEndedIterator,
	{
		Iterator::chain(
			std::iter::once(entity),
			self.iter_descendants_depth_first(entity),
		)
	}
}




/// An [`Iterator`] of [`Entity`]s over the direct descendants of an [`Entity`].
pub struct DirectDescendantIter {
	vec: Vec<Entity>,
}

impl DirectDescendantIter {
	/// Returns a new [`DDirectescendantIter`].
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
