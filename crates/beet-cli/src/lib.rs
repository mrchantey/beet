#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(exit_status_error)]

pub mod commands;
pub mod watch;

pub mod prelude {
	pub use crate::commands::*;
	pub use crate::watch::*;
}
