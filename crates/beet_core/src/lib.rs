#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

#[cfg(feature = "server")]
pub mod server;

pub mod prelude {
	#[cfg(feature = "server")]
	pub use crate::server::*;
}
