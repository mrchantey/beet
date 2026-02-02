//! Test outcome types for the test runner.
//!
//! This module defines the types used to represent the outcome of a test,
//! including pass, skip, and various failure modes.

use crate::prelude::*;
use crate::testing::runner::*;
use crate::testing::utils::*;

/// The outcome of running a test.
#[derive(Debug, Clone, PartialEq, Eq, Component)]
#[component(storage = "SparseSet")]
pub enum TestOutcome {
	/// The test either returned ok, or was expected to panic and did so.
	Pass,
	/// The test was skipped for some reason.
	Skip(TestSkip),
	/// The test failed.
	Fail(TestFail),
}


/// Reasons why a test was skipped.
///
/// Applied to test entities either upon spawn or after applying a filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestSkip {
	/// The test has a `#[no_run]` attribute.
	NoRun,
	/// The test has a `#[compile_fail]` attribute.
	CompileFail,
	/// The test has an `#[ignore]` attribute.
	Ignore(Option<&'static str>),
	/// The test was filtered out by user-specified filters.
	FailedFilter,
}

/// Reasons why a test failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestFail {
	/// The test returned an [`Err(String)`].
	Err {
		/// The error message.
		message: String,
	},
	/// The test did not panic but was expected to.
	ExpectedPanic {
		/// The expected panic message, if specified.
		message: Option<String>,
	},
	/// The test panicked unexpectedly.
	Panic {
		/// The payload downcast from the `Box<dyn Any>` panic payload,
		/// or 'opaque payload' if it couldn't be downcast.
		payload: Option<String>,
		/// The location where the panic occurred.
		location: Option<FileSpan>,
	},
	/// The test timed out.
	Timeout {
		/// How long the test ran before timing out.
		elapsed: Duration,
	},
}

impl TestFail {
	/// Gets the file path of the failure location.
	///
	/// Returns the panic location if available, otherwise the test file path.
	pub fn path(&self, test: &Test) -> WsPathBuf {
		match self {
			TestFail::Panic { location, .. }
				if let Some(location) = location =>
			{
				location.file().clone()
			}
			_ => test.path(),
		}
	}

	/// Gets the start location of the failure.
	///
	/// Returns the panic location if available, otherwise the test location.
	pub fn start(&self, test: &Test) -> LineCol {
		match self {
			TestFail::Panic { location, .. }
				if let Some(location) = location =>
			{
				location.start()
			}
			_ => test.start(),
		}
	}

	/// Gets the end location of the failure.
	///
	/// Returns the panic location if available, otherwise the test location.
	pub fn end(&self, test: &Test) -> LineCol {
		match self {
			TestFail::Panic { location, .. }
				if let Some(location) = location =>
			{
				location.end()
			}
			_ => test.end(),
		}
	}

	/// Returns `true` if this is a timeout failure.
	pub fn is_timeout(&self) -> bool {
		matches!(self, TestFail::Timeout { .. })
	}

	/// Returns `true` if this is a panic failure.
	pub fn is_panic(&self) -> bool { matches!(self, TestFail::Panic { .. }) }

	/// Returns `true` if this is an error failure.
	pub fn is_error(&self) -> bool { matches!(self, TestFail::Err { .. }) }

	/// Returns `true` if this is an expected panic failure.
	pub fn is_expected_panic(&self) -> bool {
		matches!(self, TestFail::ExpectedPanic { .. })
	}
}


impl Into<TestOutcome> for TestFail {
	fn into(self) -> TestOutcome { TestOutcome::Fail(self) }
}
impl Into<TestOutcome> for TestSkip {
	fn into(self) -> TestOutcome { TestOutcome::Skip(self) }
}

impl TestOutcome {
	/// Returns `true` if this is a pass outcome.
	pub fn is_pass(&self) -> bool { self == &TestOutcome::Pass }

	/// Returns `true` if this is a fail outcome.
	pub fn is_fail(&self) -> bool { matches!(self, TestOutcome::Fail(_)) }

	/// Returns `true` if this is a skip outcome.
	pub fn is_skip(&self) -> bool { matches!(self, TestOutcome::Skip(_)) }

	/// Returns the failure details if this is a fail outcome.
	pub fn as_fail(&self) -> Option<&TestFail> {
		if let TestOutcome::Fail(fail) = self {
			Some(fail)
		} else {
			None
		}
	}

	/// Creates a [`TestOutcome`] from a panic result and whether the test should panic.
	///
	/// The `should_panic` parameter is retrieved via [`Test::should_panic`].
	pub fn from_panic_result(
		result: PanicResult,
		should_panic: test::ShouldPanic,
	) -> Self {
		match (result, should_panic) {
			(PanicResult::Ok, test::ShouldPanic::No) => {
				//ok
				TestOutcome::Pass
			}
			(PanicResult::Ok, test::ShouldPanic::Yes) => {
				//ok but should have panicked
				TestOutcome::Fail(TestFail::ExpectedPanic { message: None })
			}
			(PanicResult::Ok, test::ShouldPanic::YesWithMessage(message)) => {
				//ok but should have panicked
				TestOutcome::Fail(TestFail::ExpectedPanic {
					message: Some(message.to_string()),
				})
			}
			(PanicResult::Err(message), _) => {
				// errored
				TestOutcome::Fail(TestFail::Err { message })
			}
			(
				PanicResult::Panic { .. },
				test::ShouldPanic::Yes | test::ShouldPanic::YesWithMessage(_),
			) => {
				// panicked and should have
				TestOutcome::Pass
			}
			(
				PanicResult::Panic { location, payload },
				test::ShouldPanic::No,
			) => {
				// panicked but shouldnt have
				TestOutcome::Fail(TestFail::Panic { location, payload })
			}
		}
	}
}
