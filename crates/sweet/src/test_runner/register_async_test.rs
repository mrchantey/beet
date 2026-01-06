use beet_core::prelude::*;
use std::cell::RefCell;
use std::pin::Pin;

thread_local! {
	/// A thread-local cell holding the currently registered async test, if any.
	/// This technique allows an opaque `fn()` provided by libtest to register
	/// an async tests.
	static REGISTERED_ASYNC_TEST: RefCell<Option<Pin<Box<dyn AsyncTest>>>> = RefCell::new(None);
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

/// A trait representing an async test future that returns a Result<(), String>
pub trait AsyncTest: 'static + Future<Output = Result<(), String>> {}
impl<F> AsyncTest for F where F: 'static + Future<Output = Result<(), String>> {}


/// Attempts to run the provided function as a synchronous test.
///
/// ## Panics
/// Panics if a test is already registered, meaning its been registered
/// outside of a provided function.
pub(super) fn try_run_async(
	func: impl FnOnce() -> Result<(), String>,
) -> MaybeAsync {
	// should already be none, but just incase somebody gets clever
	REGISTERED_ASYNC_TEST.with(|cell| {
		// *cell.borrow_mut() = None;
		if !cell.borrow().is_none() {
			panic!(
				"async test was registered outside of a test run. This is not supported"
			);
		}
	});
	let panic_outcome = PanicContext::catch(func);
	match REGISTERED_ASYNC_TEST.with(|cell| cell.borrow_mut().take()) {
		Some(async_test) => {
			MaybeAsync::Async(Box::pin(PanicContext::catch_async(async_test)))
		}
		None => MaybeAsync::Sync(panic_outcome),
	}
}


pub trait IntoFut<M> {
	fn into_fut(self) -> impl AsyncTest;
}
pub struct ReturnsResult;
pub struct ReturnsUnit;
pub struct ReturnsNever;

impl<T> IntoFut<ReturnsResult> for T
where
	T: AsyncTest,
{
	fn into_fut(self) -> impl AsyncTest { self }
}
impl<T> IntoFut<ReturnsUnit> for T
where
	T: 'static + Future<Output = ()>,
{
	fn into_fut(self) -> impl AsyncTest {
		async move {
			self.await;
			Ok(())
		}
	}
}
impl<T> IntoFut<ReturnsNever> for T
where
	T: 'static + Future<Output = !>,
{
	fn into_fut(self) -> impl AsyncTest {
		async move {
			self.await;
		}
	}
}

// see run_tests.rs for tests
