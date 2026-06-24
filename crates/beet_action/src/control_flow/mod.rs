mod call_on_spawn;
mod exclude_errors;
mod fallback;
pub use call_on_spawn::*;
pub use exclude_errors::*;
pub use fallback::*;
mod highest_score;
pub use highest_score::*;
mod parallel;
pub use parallel::*;
mod score;
pub use score::*;
mod repeat;
pub use repeat::*;
mod run_on_load;
pub use run_on_load::*;
mod running;
pub use running::*;
mod sequence;
pub use sequence::*;

use beet_core::prelude::*;

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

impl<Pass,Fail> Default for Outcome<Pass, Fail> where Pass:Default {
	fn default() -> Self { Self::Pass(default()) }
}
