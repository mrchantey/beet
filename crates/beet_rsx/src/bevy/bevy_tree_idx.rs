use crate::prelude::*;
use bevy::ecs::query::QueryIter;
use bevy::prelude::*;

/// Represents an element in the rsx tree. This may be changed
/// in the future for the tag to be its own component.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct BevyRsxElement {
	/// The element tag, ie <div>
	pub tag: String,
}

impl TreeIdx {
	pub fn find<'a>(
		query: QueryIter<(Entity, &TreeIdx), ()>,
		loc: TreeLocation,
	) -> Option<Entity> {
		// O(n) search, if we have more than a few hundred entities
		// we should consider a hashmap
		query
			.into_iter()
			.find(|(_, idx)| **idx == loc.tree_idx)
			.map(|(entity, _)| entity)
	}
	pub fn find_mut<'a>(
		query: &'a mut Query<EntityMut, With<TreeIdx>>,
		loc: TreeLocation,
	) -> Option<EntityMut<'a>> {
		// O(n) search, if we have more than a few hundred entities
		// we should consider a hashmap
		query.iter_mut().find(|entity| {
			entity
				.get::<TreeIdx>()
				.map(|idx| *idx == loc.tree_idx)
				.unwrap_or(false)
		})
	}
}


pub mod expect_rsx_element {
	use crate::prelude::*;


	pub fn to_be_at_location(location: &TreeLocation) -> String {
		format!("failed to find entity at location: {:?}", location)
	}
}
