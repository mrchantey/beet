use crate::html::DomLocation;
use crate::html::RsxIdx;
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
/// Represents an element in the rsx tree. This may be changed
/// in the future for the tag to be its own component.
#[derive(Debug, Default, Component, Reflect, Deref)]
#[reflect(Default, Component)]
pub struct BevyRsxIdx(RsxIdx);


impl BevyRsxIdx {
	pub fn new(rsx_idx: RsxIdx) -> Self { Self(rsx_idx) }

	pub fn find<'a>(
		query: QueryIter<(Entity, &BevyRsxIdx), ()>,
		loc: DomLocation,
	) -> Option<Entity> {
		// O(n) search, if we have more than a few hundred entities
		// we should consider a hashmap
		query
			.into_iter()
			.find(|(_, rsx_idx)| ***rsx_idx == loc.rsx_idx)
			.map(|(entity, _)| entity)
	}
	pub fn find_mut<'a>(
		query: &'a mut Query<EntityMut, With<BevyRsxIdx>>,
		loc: DomLocation,
	) -> Option<EntityMut<'a>> {
		// O(n) search, if we have more than a few hundred entities
		// we should consider a hashmap
		query.iter_mut().find(|entity| {
			entity
				.get::<BevyRsxIdx>()
				.map(|rsx_idx| **rsx_idx == loc.rsx_idx)
				.unwrap_or(false)
		})
	}
}


pub mod expect_rsx_element {
	use crate::html::DomLocation;


	pub fn to_be_at_location(location: &DomLocation) -> String {
		format!("failed to find entity at location: {:?}", location)
	}
}
