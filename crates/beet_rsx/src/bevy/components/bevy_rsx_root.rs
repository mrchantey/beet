use crate::prelude::*;
use bevy::prelude::Deref;
use bevy::prelude::*;



/// Added to each entity at the root of an rsx tree,
/// there may be serveral, ie if the root was a fragment.
#[derive(Debug, Default, PartialEq, Component, Deref)]
pub struct BevyRsxLocation(RsxMacroLocation);

impl BevyRsxLocation {
	pub fn new(location: RsxMacroLocation) -> Self { Self(location) }
}
