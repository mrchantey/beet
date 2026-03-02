#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![cfg_attr(not(feature = "std"), no_std)]
// #![deny(missing_docs)]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod types;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::types::*;
}
