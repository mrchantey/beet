#![doc = include_str!("../../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
pub mod collect_routes;
pub mod compile_check;
pub mod build_template_map;


pub mod prelude {
	pub use crate::collect_routes::*;
	pub use crate::compile_check::*;
	pub use crate::build_template_map::*;
}
