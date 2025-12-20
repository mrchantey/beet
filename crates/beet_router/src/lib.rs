#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(if_let_guard)]
mod actions;
mod types;

pub mod prelude {
	pub use crate::actions::*;
	pub use crate::types::*;
}

