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
