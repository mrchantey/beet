use bevy::prelude::*;
use std::hash::Hash;



/// Added to each [`LangPartial`] and each [`NodePortalTarget`] source,
/// in the case of a [`StyleScope::Local`].
/// The propagation of style ids to each element in a template is performed
/// at a later stage.
#[derive(
	Debug, Copy, Clone, PartialEq, Eq, Hash, Deref, Component, Reflect,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[component(immutable)]
pub struct StyleId(u64);
impl StyleId {
	/// Create a new [`StyleId`] from a `u64`.
	pub fn new(id: u64) -> Self { Self(id) }
}
impl std::fmt::Display for StyleId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
