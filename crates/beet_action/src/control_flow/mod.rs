mod fallback;
pub use fallback::*;
mod repeat;
pub use repeat::*;
mod sequence;
pub use sequence::*;

use beet_core::prelude::*;
use bitflags::bitflags;

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

/// A control flow return type that can be used to implement
/// fallback/sequence, if/else,
/// switch, and other control flow structures.
///
/// Outcome payloads default to `()` and variants are exposed at the crate level:
/// ```rust
/// # use beet_action::prelude::*;
///
/// fn short() -> Outcome {
/// 	Pass(())
/// }
/// // is the same as
/// fn explicit() -> Outcome<(),()> {
/// 	Outcome::Pass(())
/// }
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Outcome<P = (), F = ()> {
	Pass(P),
	Fail(F),
}

impl Outcome<(), ()> {
	pub const FAIL: Self = Self::Fail(());
	pub const PASS: Self = Self::Pass(());
}
