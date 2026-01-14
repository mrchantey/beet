use crate::prelude::*;
use beet_core::prelude::*;
use std::cell::RefCell;
use std::pin::Pin;

thread_local! {
	/// A thread-local cell holding the currently registered async test, if any.
	/// This technique allows an opaque `fn()` provided by libtest to register
	/// an async tests.
	static REGISTERED_ASYNC_TEST: RefCell<Option<Pin<Box<dyn AsyncTest>>>> = RefCell::new(None);

	/// A thread-local cell holding test parameters registered by the test macro.
	static REGISTERED_TEST_PARAMS: RefCell<Option<TestCaseParams>> = RefCell::new(None);
}

/// Called by the [`sweet::test`] macro to register test parameters.
#[track_caller]
pub fn register_test_params(params: TestCaseParams) {
	REGISTERED_TEST_PARAMS.with(|cell| {
		*cell.borrow_mut() = Some(params);
	});
}

/// Called by the [`sweet::test`] macro in the case its provided an async
/// function.
#[track_caller]
pub fn register_async_test<M>(fut: impl IntoFut<M>) {
	REGISTERED_ASYNC_TEST.with(|cell| {
		*cell.borrow_mut() = Some(Box::pin(fut.into_fut()));
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
	pub params: Option<TestCaseParams>,
}


/// Attempts to run the provided function as a synchronous test.
///
/// ## Panics
/// Panics if a test is already registered, meaning its been registered
/// outside of a provided function.
pub(super) fn try_run_async(
	func: impl FnOnce() -> Result<(), String>,
) -> TestRunResult {
	// should already be none, but just incase somebody gets clever
	REGISTERED_ASYNC_TEST.with(|cell| {
		// *cell.borrow_mut() = None;
		if !cell.borrow().is_none() {
			panic!(
				"async test was registered outside of a test run. This is not supported"
			);
		}
	});
	REGISTERED_TEST_PARAMS.with(|cell| {
		if !cell.borrow().is_none() {
			panic!(
				"test params were registered outside of a test run. This is not supported"
			);
		}
	});

	let panic_outcome = PanicContext::catch(func);

	let params = REGISTERED_TEST_PARAMS.with(|cell| cell.borrow_mut().take());
	let maybe_async =
		match REGISTERED_ASYNC_TEST.with(|cell| cell.borrow_mut().take()) {
			Some(async_test) => MaybeAsync::Async(Box::pin(
				PanicContext::catch_async(async_test),
			)),
			None => MaybeAsync::Sync(panic_outcome),
		};

	TestRunResult {
		maybe_async,
		params,
	}
}

// see run_tests.rs for tests
