use crate::prelude::*;
use bevy::prelude::Deref;
use bevy::prelude::*;



/// Added to each entity at the root of an rsx tree,
/// there may be serveral, ie if the root was a fragment.
#[derive(Debug, Default, PartialEq, Component, Deref)]
pub struct BevyRsxRoot(RsxLocation);

impl BevyRsxRoot {
	pub fn new(location: RsxLocation) -> Self { Self(location) }
}
