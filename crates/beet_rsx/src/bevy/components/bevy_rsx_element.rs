use crate::html::DomLocation;
use crate::html::RsxIdx;
use bevy::prelude::*;


/// Represents an element in the rsx tree. This may be changed
/// in the future for the tag to be its own component.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct BevyRsxElement {
	/// The element tag, ie <div>
	pub tag: String,
	/// The rsx idx applied when visiting this node
	pub idx: RsxIdx,
}


impl BevyRsxElement {
	pub fn find_mut<'a>(
		query: &'a mut Query<EntityMut, With<BevyRsxElement>>,
		loc: DomLocation,
	) -> Option<EntityMut<'a>> {
		// O(n) search, if we have more than a few hundred entities
		// we should consider a hashmap

		query.iter_mut().find(|entity| {
			entity
				.get::<BevyRsxElement>()
				.map(|rsx| rsx.idx == loc.rsx_idx)
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
