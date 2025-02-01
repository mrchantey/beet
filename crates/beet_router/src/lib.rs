#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]


pub mod file_routes;
pub mod utils;


pub mod prelude {
	pub use crate::file_routes::*;
	pub use crate::utils::*;
}
