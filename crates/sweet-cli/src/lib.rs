#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
pub mod automod;
pub mod bench;
pub mod test_runners;

pub mod prelude {
	pub use crate::automod::*;
	pub use crate::bench::*;
	pub use crate::test_runners::*;
}
