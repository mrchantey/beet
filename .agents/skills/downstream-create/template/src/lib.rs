//! @description@
// the harness main for `cargo test --lib`; cfg gated so a plain build does not
// need the facade's `testing` feature
#[cfg(test)]
beet::test_main!();

mod plugin;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::plugin::*;
}
