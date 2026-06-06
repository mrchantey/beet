#![doc = include_str!("../README.md")]

#[cfg(test)]
beet::test_main!();

mod commands;

pub mod prelude {
	pub use crate::commands::*;
}
