//! Opt-outs from the failures control flow raises, one component per mechanism.
use beet_core::prelude::*;
use bitflags::bitflags;

/// Which child errors to exclude, defaults to none.
#[derive(Debug, Default, Clone, Copy, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct ExcludeErrors(pub ChildError);

bitflags! {
	/// Child error types that can occur during control-flow execution.
	/// Used with [`ExcludeErrors`] to selectively skip certain child issues.
	#[repr(transparent)]
	#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
	#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
	#[reflect(opaque)]
	#[reflect(Hash, Clone, PartialEq, Debug, Default)]
	#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
	pub struct ChildError: u8 {
		/// Child entity has no [`ActionMeta`](crate::prelude::ActionMeta) component.
		const NO_ACTION = 0b01;
		/// Child entity has an action with an incompatible signature.
		const ACTION_MISMATCH = 0b10;
		/// Every child was skipped, so a parent that has children would run none
		/// of them. Excluded only by a parent for which doing nothing is a valid
		/// outcome.
		const NONE_VALID = 0b100;
	}
}

/// Which [`RunningSet`](crate::prelude::RunningSet) errors to exclude, defaults to none.
///
/// An entity whose facets may all decline yet which should still park (a boot
/// whose selection named none of them) declares the opt-out here, rather than
/// the set second-guessing what a caller meant by an empty start.
#[derive(Debug, Default, Clone, Copy, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct ExcludeRunningErrors(pub RunningError);

bitflags! {
	/// Failures a [`RunningSet`](crate::prelude::RunningSet) resolves its parked call with.
	/// Used with [`ExcludeRunningErrors`] to park instead.
	#[repr(transparent)]
	#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
	#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
	#[reflect(opaque)]
	#[reflect(Hash, Clone, PartialEq, Debug, Default)]
	#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
	pub struct RunningError: u8 {
		/// Every declared facet declined the start, so nothing holds the run open.
		const NONE_STARTED = 0b01;
	}
}
