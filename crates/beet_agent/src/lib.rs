#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

mod providers;
pub mod realtime;
mod session;


pub mod prelude {
	pub use crate::providers::*;
	pub use crate::realtime;
	pub use crate::session::*;
}


#[cfg(test)]
pub use crate::session::test_utils;
