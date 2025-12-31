#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(crate::test_runner))]
// #![feature(test)]
// remove after bevy refactor
#![allow(deprecated)]
// #![deny(missing_docs)]
#![doc = include_str!("../README.md")]
// implement FnMut for MockFunc
#![cfg_attr(
	feature = "nightly",
	feature(
		never_type,
		fn_traits,
		backtrace_frames,
		unboxed_closures,
		if_let_guard,
		test,
		closure_track_caller
	)
)]

extern crate test;
// the #[sweet::test] macro
pub use bevy_runner::test_runner;
pub use sweet_macros;
pub use sweet_macros::test;
pub mod bevy_runner;
/// Utilities for [libtest](https://github.com/rust-lang/rust/tree/master/library/test)
pub mod libtest;
pub mod utils;

/// Matchers used for assertions: `true.xpect_true()`
mod matchers;

pub mod prelude {
	pub use crate::bevy_runner::*;
	pub use crate::libtest::*;
	pub use crate::matchers::*;
	pub use crate::utils::*;
}

pub mod as_sweet {
	pub use crate::prelude::*;
	pub mod sweet {
		pub use crate::exports;
		pub use crate::prelude;
	}
}

pub mod exports {}
