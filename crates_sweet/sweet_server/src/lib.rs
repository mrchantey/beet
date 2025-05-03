#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet_test::test_runner))]

pub mod server;
pub mod prelude {
	pub use crate::server::*;
}
