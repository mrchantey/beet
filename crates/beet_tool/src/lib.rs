// #![deny(missing_docs)]

mod control_flow;
mod tools;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::control_flow::Outcome::Fail;
	pub use crate::control_flow::Outcome::Pass;
	pub use crate::control_flow::*;
	pub use crate::tools::*;
}
