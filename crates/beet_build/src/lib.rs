//! This crate enables various forms of code generation for beet crates,
//! and can also be used as a standalone library.
//! It includes very large parsers but all are gated by feature flags.
#![feature(let_chains)]


mod utils;

pub mod prelude {
	pub use crate::utils::*;
}
