#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(crate::test_runner))]
// #![feature(test)]
// remove after bevy refactor
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
// Allow the crate to reference itself as `sweet::` in tests,
// required for the `#[sweet::test]` macro to use the correct thread local
#[cfg(test)]
extern crate self as sweet;
// the #[sweet::test] macro
pub use sweet_macros;
pub use sweet_macros::test;
#[cfg(feature = "runner")]
pub use test_runner::test_runner;
mod matchers;
#[cfg(feature = "runner")]
mod test_runner;

pub mod prelude {
	pub use crate::matchers::*;
	#[cfg(feature = "runner")]
	pub use crate::test_runner::*;
}

pub mod exports {}
