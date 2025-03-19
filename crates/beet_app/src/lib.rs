#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

mod app_router;
mod beet_app_args;
#[cfg(feature = "serde")]
mod file_group;
mod collections;
mod root_context;

pub mod prelude {
	pub use crate::app_router::*;
	pub use crate::beet_app_args::*;
	#[cfg(feature = "serde")]
	pub use crate::file_group::*;
	pub use crate::collections::*;
	pub use crate::root_context::*;
	pub use crate::root_cx;
}
