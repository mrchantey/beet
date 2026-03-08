#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

mod media;


/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::media::*;
}
