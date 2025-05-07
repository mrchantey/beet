#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

//! low dependency common types and helpers for beet_rsx.
//! This crate is the top level for beet_rsx, beet_rsx_parser etc.
#![feature(let_chains)]

#[cfg(feature = "tokens")]
pub mod tokens_utils;

pub mod prelude {
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
}
