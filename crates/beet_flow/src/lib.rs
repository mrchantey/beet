#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
// #![deny(missing_docs)]
// #![doc = include_str!("../README.md")]
#[cfg(feature = "bevy_default")]
#[allow(unused, reason = "docs")]
use crate::prelude::*;

mod actions;
mod events;
mod types;

pub mod prelude {
	pub use crate::actions::*;
	pub use crate::events::*;
	pub use crate::types::*;
}
