use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::query::WorldQuery;
use bevy::prelude::Children;
use bevy::prelude::Entity;
use bevy::prelude::HierarchyQueryExt;
use bevy::prelude::Parent;
// use bevy::prelude::*;


pub trait HierarchyQueryExt2<'w, 's, D: QueryData, F: QueryFilter>:
	HierarchyQueryExt<'w, 's, D, F>
{
	/// iterate descendants including the root entity
	fn iter_descendants_inclusive(
		&'w self,
		entity: Entity,
	) -> impl Iterator<Item = Entity> + 'w
	where
		's: 'w,
		D: 'w + 's,
		F: 'w + 's,
		D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
	{
		std::iter::once(entity).chain(self.iter_descendants(entity))
	}
	/// iterate ancestors including the root entity
	fn iter_ancestors_inclusive(
		&'w self,
		entity: Entity,
	) -> impl Iterator<Item = Entity> + 'w
	where
		's: 'w,
		D: 'w + 's,
		F: 'w + 's,
		D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
	{
		std::iter::once(entity).chain(self.iter_ancestors(entity))
	}
}

impl<'w, 's, D: QueryData, F: QueryFilter, T> HierarchyQueryExt2<'w, 's, D, F>
	for T
where
	T: HierarchyQueryExt<'w, 's, D, F>,
{
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::ecs::system::SystemState;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		let entity = world.spawn_empty().with_child(()).id();
		world.spawn_empty().add_children(&[entity]);

		let mut state = SystemState::<Query<&Children>>::new(&mut world);
		let state = state.get(&world);
		expect(state.iter_descendants(entity).count()).to_be(1);
		expect(state.iter_descendants_inclusive(entity).count()).to_be(2);

		let mut state = SystemState::<Query<&Parent>>::new(&mut world);
		let state = state.get(&world);
		expect(state.iter_ancestors(entity).count()).to_be(1);
		expect(state.iter_ancestors_inclusive(entity).count()).to_be(2);
	}
}
