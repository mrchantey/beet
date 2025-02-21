#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
pub mod components;
pub mod plugins;
pub mod scenes;

pub mod prelude {
	pub use crate::components::*;
	pub use crate::plugins::*;
}
