#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "nightly", feature(closure_track_caller))]

beet_core::test_main!();

pub mod o11s;
mod partial;
// Every provider is a set of constructors for the two streamers, one of which is
// the async-openai-backed `CompletionsStreamer`.
#[cfg(feature = "agent")]
mod providers;
pub mod realtime;
mod streaming;
pub mod table;
mod tool;
mod types;
#[cfg(feature = "ui")]
pub mod ui;

pub mod prelude {
	pub use crate::o11s;
	pub use crate::partial::*;
	#[cfg(feature = "agent")]
	pub use crate::providers::*;
	pub use crate::realtime;
	pub use crate::streaming::*;
	pub use crate::table::*;
	pub use crate::tool::*;
	pub use crate::types::*;
	#[cfg(feature = "ui")]
	pub use crate::ui::*;
}
