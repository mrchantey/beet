#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

mod beet_app;
mod beet_app_args;
mod collections;
mod root_context;

pub mod prelude {
	pub use crate::root_cx;
	pub use crate::beet_app::*;
	pub use crate::beet_app_args::*;
	pub use crate::collections::*;
	pub use crate::root_context::*;
}
