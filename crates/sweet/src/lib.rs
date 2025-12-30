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
pub use sweet_macros;
pub use sweet_macros::test;

use crate::prelude::TestPlugin;
use crate::prelude::tests_bundle_borrowed;
pub mod bevy_runner;
// #[cfg(test)]
// use libtest_runner::testlib_runner as libtest_runner;
pub mod backtrace;
pub mod bevy;
/// Utilities for [libtest](https://github.com/rust-lang/rust/tree/master/library/test)
pub mod libtest;
/// Cross platform logging utils
pub mod logging;
/// Test runner module
pub mod test_runner;
pub mod utils;

/// Matchers used for assertions: `true.xpect_true()`
mod matchers;
#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub mod native;
pub mod test_case;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod prelude {

	pub use crate::backtrace::*;
	pub use crate::bevy::*;
	pub use crate::bevy_runner::*;
	pub use crate::libtest::*;
	pub use crate::logging::*;
	pub use crate::matchers::*;
	#[cfg(not(target_arch = "wasm32"))]
	pub use crate::native::*;
	pub use crate::test_case::*;
	pub use crate::test_runner::*;
	pub use crate::utils::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::wasm::*;
}

pub mod as_sweet {
	pub use crate::prelude::*;
	pub mod sweet {
		pub use crate::exports;
		pub use crate::prelude;
	}
}

pub mod exports {
	pub use anyhow::Result;
}

/// Entry point for the sweet test runner
pub fn test_runner(tests: &[&test::TestDescAndFn]) {
	#[cfg(target_arch = "wasm32")]
	let result = crate::wasm::run_libtest_wasm(tests);
	#[cfg(not(target_arch = "wasm32"))]
	let result = crate::native::run_libtest_native(tests);
	if let Err(e) = result {
		eprintln!("Test runner failed: {e}");
		std::process::exit(1);
	}
}


pub fn test_runner2(tests: &[&test::TestDescAndFn]) {
	use beet_core::prelude::*;
	use beet_net::prelude::Request;


	App::new()
		.add_plugins((MinimalPlugins, TestPlugin))
		.spawn_then((
			Request::from_cli_args(CliArgs::parse_env()).unwrap_or_exit(),
			tests_bundle_borrowed(tests),
		))
		.run();
}
