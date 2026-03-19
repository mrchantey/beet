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
	#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
	pub struct ChildError: u8 {
		/// Child entity has no [`ToolMeta`] component.
		const NO_TOOL = 0b01;
		/// Child entity has a tool with an incompatible signature.
		const TOOL_MISMATCH = 0b10;
	}
}

/// A control flow return type that can be used to implement
/// fallback/sequence, if/else,
/// switch, and other control flow structures.
///
/// Outcome payloads default to `()` and variants are exposed at the crate level:
/// ```rust
/// # use beet_tool::prelude::*;
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
