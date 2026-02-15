mod fallback;
pub use fallback::*;

use beet_core::prelude::*;

/// A control flow return type that can be used to implement
/// fallback/sequence, if/else,
/// switch, and other control flow structures.
/// Outcome payloads default to `()` and variants are exposed at the crate level:
/// ```rust
/// # use beet_stack::prelude::*;
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
