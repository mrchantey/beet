//!
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![cfg_attr(feature = "nightly", feature(closure_track_caller))]

pub mod o11s;
mod partial;
mod providers;
pub mod realtime;
mod streaming;
pub mod table;
mod tool;
mod types;


pub mod prelude {
	pub use crate::o11s;
	pub use crate::partial::*;
	pub use crate::providers::*;
	pub use crate::realtime;
	pub use crate::streaming::*;
	pub use crate::table::*;
	pub use crate::tool::*;
	pub use crate::types::*;
}
