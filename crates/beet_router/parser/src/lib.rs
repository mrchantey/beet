#![doc = include_str!("../../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
pub mod compile_check;
pub mod parse_file_router;
pub mod parse_route_file;


pub mod prelude {
	pub use crate::compile_check::*;
	pub use crate::parse_file_router::*;
	pub use crate::parse_route_file::*;
}
