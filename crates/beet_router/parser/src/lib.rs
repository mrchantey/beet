#![doc = include_str!("../../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
pub mod compile_check;
pub mod routes_mod;


pub mod prelude {
	pub use crate::compile_check::*;
	pub use crate::routes_mod::*;
}
