//!
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![cfg_attr(feature = "nightly", feature(closure_track_caller))]

pub mod document;
pub mod o11s;
mod providers;
pub mod realtime;
mod tool;
mod types;


pub mod prelude {
	pub use crate::document::*;
	pub use crate::o11s;
	pub use crate::providers::*;
	pub use crate::realtime;
	pub use crate::tool::*;
	pub use crate::types::*;
}
