//! Sweet test registration mechanism.
//!
//! This module provides the unified registration system for sweet tests.
//!
//! ## Registration Patterns
//!
//! ### Vanilla tests (`#[test]`)
//! - No registration, runs directly via libtest
//! - Standard sync test execution
//!
//! ### Sweet tests (`#[beet_core::test]`)
//! - Unified registration via `register_sweet_test()`
//! - Registers both `TestCaseParams` AND test future in a single call
//! - Thread-local storage allows opaque `fn()` from libtest to provide async tests
//! - Params are available before awaiting the future, enabling:
//!   - Per-test timeout configuration
//!   - Future extensibility (retries, resource limits, etc.)
//!   - ECS component inspection before test execution
//!
//! ## Design Philosophy
//!
//! The key insight is separating params from execution. By registering both together
//! but extracting them separately, we can inspect/use params before running the test.
//! This is crucial for features like timeout enforcement where we need to know the
//! timeout value before starting the async test.
//!
//! ## How it works
//!
//! 1. `#[beet_core::test]` macro generates code calling `register_sweet_test(params, async_body)`
//! 2. Both params and test are stored together in thread-local `REGISTERED_SWEET_TEST`
//! 3. Test runner calls `try_run_async()` which:
//!    - Invokes the test function (triggering registration)
//!    - Extracts both params and async test from thread-local
//!    - Returns `TestRunResult { maybe_async, params }`
//! 4. Params are inserted as ECS components and used immediately
//! 5. Test future is awaited separately with params already available

use crate::prelude::*;
use crate::testing::runner::*;
use crate::testing::utils::*;
use std::cell::RefCell;
use std::pin::Pin;

thread_local! {
	/// Thread-local storage for sweet test registration.
	/// When a test function is invoked by libtest, it calls `register_sweet_test`
	/// which populates this cell with both the async test future and params.
	static REGISTERED_SWEET_TEST: RefCell<Option<SweetTestRegistration>> = RefCell::new(None);
}

/// Registration data for a sweet test, containing both the test future and params
struct SweetTestRegistration {
	async_test: Pin<Box<dyn AsyncTest>>,
	params: TestCaseParams,
}

/// Unified registration for sweet tests - registers both params and test future.
///
/// Called by the `#[beet_core::test]` macro to register test configuration and the
/// async test body in a single call. This ensures params are available before
/// the test future is awaited, enabling future extensibility (e.g., conditional
/// execution, resource allocation, etc.).
///
/// ## Example
///
/// The macro:
/// ```ignore
/// #[beet_core::test(timeout_ms = 1000)]
/// async fn my_test() {
///     assert!(true);
/// }
/// ```
///
/// Expands to:
/// ```ignore
/// #[test]
/// fn my_test() {
///     beet_core::testing::register_sweet_test(
///         beet_core::testing::TestCaseParams::new().with_timeout_ms(1000),
///         async { assert!(true); }
///     );
/// }
/// ```
///
/// This allows the test runner to:
/// 1. Extract params before awaiting the future
/// 2. Use params for timeout enforcement, retries, etc.
/// 3. Insert params as ECS components for system access
#[track_caller]
pub fn register_sweet_test<M>(params: TestCaseParams, fut: impl IntoFut<M>) {
	REGISTERED_SWEET_TEST.with(|cell| {
		*cell.borrow_mut() = Some(SweetTestRegistration {
			async_test: Box::pin(fut.into_fut()),
			params,
		});
	});
}

/// Legacy function for registering test parameters separately.
/// Prefer `register_sweet_test` which registers both at once.
#[track_caller]
pub fn register_test_params(params: TestCaseParams) {
	REGISTERED_SWEET_TEST.with(|cell| {
		let mut registration = cell.borrow_mut();
		if let Some(reg) = registration.as_mut() {
			reg.params = params;
		} else {
			*registration = Some(SweetTestRegistration {
				async_test: Box::pin(async { Ok(()) }),
				params,
			});
		}
	});
}

/// Represents either an async test that is still running,
/// or a synchronous test that has finished.
pub(super) enum MaybeAsync {
	/// The test is async and hasnt yet finished
	Async(Pin<Box<dyn Future<Output = PanicResult>>>),
	/// The test is sync and has finished running
	Sync(PanicResult),
}

/// Result of running a test function, including both outcome and params
pub(super) struct TestRunResult {
	pub maybe_async: MaybeAsync,
	pub params: TestCaseParams,
}

/// Attempts to run the provided function as a synchronous test.
/// If the function registers a sweet test via `register_sweet_test` or legacy methods,
/// returns the async test future and params. Otherwise runs as sync test.
///
/// ## Panics
/// Panics if a test is already registered outside of a test run.
pub(super) fn try_run_async(
	func: impl FnOnce() -> Result<(), String>,
) -> TestRunResult {
	REGISTERED_SWEET_TEST.with(|cell| {
		if cell.borrow().is_some() {
			panic!(
				"sweet test was registered outside of a test run. This is not supported"
			);
		}
	});

	let panic_outcome = PanicContext::catch(func);

	let registration =
		REGISTERED_SWEET_TEST.with(|cell| cell.borrow_mut().take());

	match registration {
		Some(reg) => {
			let async_test = PanicContext::catch_async(reg.async_test);
			TestRunResult {
				maybe_async: MaybeAsync::Async(Box::pin(async_test)),
				params: reg.params,
			}
		}
		None => TestRunResult {
			maybe_async: MaybeAsync::Sync(panic_outcome),
			params: TestCaseParams::new(),
		},
	}
}

// see run_tests.rs for tests
