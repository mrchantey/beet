use beet_core::prelude::*;
use bitflags::bitflags;

/// Which errors to exclude, defaults to none.
#[derive(Debug, Default, Clone, Copy, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct ExcludeErrors(pub ChildError);

impl ExcludeErrors {
	pub fn all() -> Self {
		Self(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH)
	}
}

bitflags! {
	/// Child error types that can occur during control-flow execution.
	/// Used with `exclude_errors` to selectively skip certain child issues.
	#[repr(transparent)]
	#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
	#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
	#[reflect(opaque)]
	#[reflect(Hash, Clone, PartialEq, Debug, Default)]
	#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
	pub struct ChildError: u8 {
		/// Child entity has no [`ActionMeta`] component.
		const NO_ACTION = 0b01;
		/// Child entity has an action with an incompatible signature.
		const ACTION_MISMATCH = 0b10;
	}
}
