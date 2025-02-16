use bevy::prelude::Deref;
use bevy::prelude::DerefMut;
use bevy::prelude::*;


/// Represents an element in the rsx tree. This may be changed
/// in the future for the tag to be its own component.
#[derive(Debug, Default, Component, Reflect, Deref, DerefMut)]
#[reflect(Default, Component)]
pub struct ElementTag {
	pub tag: String,
}
