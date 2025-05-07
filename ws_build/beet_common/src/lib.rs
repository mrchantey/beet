//! low dependency common types and helpers for beet crates.
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

#![feature(let_chains)]

pub mod node;
#[cfg(feature = "tokens")]
pub mod tokens_utils;

pub mod prelude {
	pub use crate::node::*;
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
}
