#[cfg(feature = "tokens")]
use crate::as_beet::*;
use bevy::prelude::*;

/// A unique identifier for this node in a templating tree,
/// used for dom binding and reconcilling client island locations.
/// The index must be assigned in a depth-first manner so that
/// client islands can resolve their child indices.
///
/// A DomIdx is assigned to each [`ElementNode`] in the tree requiring
/// a binding, and each [`TemplateNode`] with a client island directive.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct DomIdx(
	/// Depth-first assigned index of this node in the templating tree.
	pub u32,
);

impl std::fmt::Display for DomIdx {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "DomIdx({})", self.0)
	}
}

impl DomIdx {
	pub fn new(idx: u32) -> Self { Self(idx) }
	pub fn inner(&self) -> u32 { self.0 }
}

/// Marker type indicating this entity will need a [`DomIdx`]
/// assigned to it after the tree has been built.
/// This is often a [`RequiredComponent`] ie [`EventTarget`] or [`ClientOnlyDirective`],
/// or added to a parent OnSpawn (ie for signal attributes)
#[derive(Default, Clone, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct RequiresDomIdx;
