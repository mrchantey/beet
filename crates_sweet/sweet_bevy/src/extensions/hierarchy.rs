use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::relationship::AncestorIter;
use bevy::ecs::relationship::DescendantIter;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use std::iter::Chain;



#[extend::ext(name=HierarchyQueryExtExt)]
pub impl<
	'w,
	's,
	D: QueryData,
	F: QueryFilter,
	// T: HierarchyQueryExt<'w, 's, D, F>,
> Query<'w, 's, D, F>
{
	/// Iterates over all ancestors of the given entity, including the entity itself.
	fn iter_ancestors_inclusive<R: Relationship>(
		&'w self,
		entity: Entity,
	) -> Chain<std::iter::Once<Entity>, AncestorIter<'w, 's, D, F, R>>
	where
		D::ReadOnly: QueryData<Item<'w> = &'w R>,
	{
		Iterator::chain(std::iter::once(entity), self.iter_ancestors(entity))
	}

	/// Iterates over all descendants of the given entity, including the entity itself.
	fn iter_descendants_inclusive<S: RelationshipTarget>(
		&'w self,
		entity: Entity,
	) -> Chain<std::iter::Once<Entity>, DescendantIter<'w, 's, D, F, S>>
	where
		D::ReadOnly: QueryData<Item<'w> = &'w S>,
	{
		Iterator::chain(std::iter::once(entity), self.iter_descendants(entity))
	}
}
