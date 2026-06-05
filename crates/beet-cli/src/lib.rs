#![doc = include_str!("../README.md")]

mod commands;

pub mod prelude {
	pub use crate::commands::*;
}
