#![doc = include_str!("../README.md")]

beet_core::test_main!();

mod commands;
mod scene_management;

pub mod prelude {
	pub use crate::commands::*;
	pub use crate::scene_management::*;
}
