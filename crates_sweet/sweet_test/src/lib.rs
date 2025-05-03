#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(crate::test_runner))]
#![feature(test)]
// #![deny(missing_docs)]
#![doc = include_str!("../README.md")]
#![feature(panic_payload_as_str)]
// implement FnMut for MockFunc
#![feature(unboxed_closures)]
#![cfg_attr(feature = "nightly", feature(fn_traits))]
// #![feature(panic_payload_as_str)]

/// Matchers and utilities for running webdriver tests
#[cfg(all(feature = "e2e", not(target_arch = "wasm32")))]
pub mod e2e;

extern crate test;
// the #[sweet::test] macro
pub use sweet_test_macros;
pub use sweet_test_macros::test;
// #[cfg(test)]
// use libtest_runner::testlib_runner as libtest_runner;
pub mod backtrace;
#[cfg(feature = "bevy")]
pub mod bevy;
/// Utilities for [libtest](https://github.com/rust-lang/rust/tree/master/library/test)
pub mod libtest;
/// Cross platform logging utils
pub mod logging;
/// Test runner module
pub mod test_runner;
pub mod utils;

#[path = "_matchers/mod.rs"]
/// Matchers used for assertions: `expect(true).to_be_true()`
pub mod matchers;
#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub mod native;
pub mod test_case;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod prelude {
	pub use crate::backtrace::*;
	#[cfg(feature = "bevy")]
	pub use crate::bevy::*;
	#[cfg(all(feature = "e2e", not(target_arch = "wasm32")))]
	pub use crate::e2e::*;
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
	#[cfg(all(feature = "e2e", not(target_arch = "wasm32")))]
	pub use fantoccini::Client;
	#[cfg(all(feature = "e2e", not(target_arch = "wasm32")))]
	pub use fantoccini::Locator;
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
