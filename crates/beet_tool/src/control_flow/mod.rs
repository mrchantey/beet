mod fallback;
pub use fallback::*;
mod repeat;
pub use repeat::*;
mod sequence;
pub use sequence::*;

use beet_core::prelude::*;

/// Policy for handling child tool mismatches during control-flow execution.
#[derive(
	Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ChildMismatch {
	/// Error on any child tool issue.
	#[default]
	Any,
	/// Error if child exists but has no tool.
	NoTool,
	/// Error if child has a tool but with the wrong signature.
	WrongTool,
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
