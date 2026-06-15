//! Test registration mechanism.
//!
//! This module provides the unified registration system for tests.
//!
//! ## Registration Patterns
//!
//! ### Vanilla tests (`#[test]`)
//! - No registration, runs directly via libtest
//! - Standard sync test execution
//!
//! ### Beet tests (`#[beet::test]`)
//! - Unified registration via `register_test()`
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
//! 1. `#[beet_core::test]` macro generates code calling `register_test(params, async_body)`
//! 2. Both params and test are stored together in thread-local `REGISTERED_TEST`
//! 3. Test runner calls `try_run_async()` which:
//!    - Invokes the test function (triggering registration)
//!    - Extracts both params and async test from thread-local
//!    - Returns `TestRunResult { maybe_async, params }`
//! 4. Params are inserted as ECS components and used immediately
//! 5. Test future is awaited separately with params already available

use crate::prelude::*;
use crate::testing::runner::*;
use crate::testing::utils::*;
use core::cell::RefCell;
use core::pin::Pin;

// Registration storage. A test fn invoked synchronously by the runner may call
// `register_test`, stashing the async body + params for the runner to pick up
// immediately after. std uses a `thread_local!`; the single-threaded esp-rtos
// executor uses a `critical_section::Mutex` static instead, which is sound
// because only the executor ever touches it (no other thread or interrupt does).
#[cfg(feature = "std")]
thread_local! {
	static REGISTERED_TEST: RefCell<Option<TestRegistration>> = RefCell::new(None);
}

// The registration holds a `Pin<Box<dyn AsyncTest>>`, which is neither `Send`
// nor `Sync`, but a `static` must be `Sync`. The single-threaded esp-rtos
// executor is the only accessor and every access is inside a critical section,
// so asserting `Sync` on the wrapper is sound.
#[cfg(not(feature = "std"))]
struct SyncRegistration(
	critical_section::Mutex<RefCell<Option<TestRegistration>>>,
);
#[cfg(not(feature = "std"))]
unsafe impl Sync for SyncRegistration {}

#[cfg(not(feature = "std"))]
static REGISTERED_TEST: SyncRegistration =
	SyncRegistration(critical_section::Mutex::new(RefCell::new(None)));

/// Stores a pending registration, clobbering any prior one.
#[cfg(feature = "std")]
fn store_registration(reg: TestRegistration) {
	REGISTERED_TEST.with(|cell| *cell.borrow_mut() = Some(reg));
}
#[cfg(not(feature = "std"))]
fn store_registration(reg: TestRegistration) {
	critical_section::with(|cs| {
		*REGISTERED_TEST.0.borrow(cs).borrow_mut() = Some(reg)
	});
}

/// Takes the pending registration, if any.
#[cfg(feature = "std")]
fn take_registration() -> Option<TestRegistration> {
	REGISTERED_TEST.with(|cell| cell.borrow_mut().take())
}
#[cfg(not(feature = "std"))]
fn take_registration() -> Option<TestRegistration> {
	critical_section::with(|cs| REGISTERED_TEST.0.borrow(cs).borrow_mut().take())
}

/// Whether a registration is currently pending.
#[cfg(feature = "std")]
fn has_registration() -> bool {
	REGISTERED_TEST.with(|cell| cell.borrow().is_some())
}
#[cfg(not(feature = "std"))]
fn has_registration() -> bool {
	critical_section::with(|cs| REGISTERED_TEST.0.borrow(cs).borrow().is_some())
}

/// Registration data for a test, containing both the test future and params
struct TestRegistration {
	async_test: Pin<Box<dyn AsyncTest>>,
	params: TestCaseParams,
}

/// Unified registration for tests - registers both params and test future.
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
///     beet_core::testing::register_test(
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
pub fn register_test<M>(params: TestCaseParams, fut: impl IntoFut<M>) {
	store_registration(TestRegistration {
		async_test: Box::pin(fut.into_fut()),
		params,
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
/// If the function registers a test via `register_test` or legacy methods,
/// returns the async test future and params. Otherwise runs as sync test.
///
/// ## Panics
/// Panics if a test is already registered outside of a test run.
pub(super) fn try_run_async(
	func: impl FnOnce() -> Result<(), String>,
) -> TestRunResult {
	if has_registration() {
		panic!(
			"test was registered outside of a test run. This is not supported"
		);
	}

	let panic_outcome = PanicContext::catch(func);

	let registration = take_registration();

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
