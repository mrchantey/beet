use crate::prelude::*;
use beet_core::prelude::*;

/// the error message
#[derive(Debug, Clone, PartialEq, Eq, Component)]
#[component(storage = "SparseSet")]
pub enum TestOutcome {
	/// The test either returned ok, or was expected to panic and did so
	Pass,
	Skip(TestSkip),
	Fail(TestFail),
}


/// Skip Outcome applied to test entities either
/// upon spawn or after applying a filter
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestSkip {
	/// The test has a `#[no_run]` attribute
	NoRun,
	/// The test has a `#[compile_fail]` attribute
	CompileFail,
	/// The test has an `#[ignore]` attribute
	Ignore(Option<&'static str>),
	/// The test was filtered out by user-specified filters
	FailedFilter,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestFail {
	/// The test returned an [`Err(String)`]
	Err { message: String },
	/// The test did not panic but was expected to
	ExpectedPanic { message: Option<String> },
	/// The test panicked
	Panic {
		/// The payload downcast from the `Box<dyn Any>`
		/// panic payload, or 'opaque payload'
		payload: Option<String>,
		/// The location of the panic if available
		location: Option<FileSpan>,
	},
}

impl TestFail {
	/// Gets the start location of the failure,
	/// or the test location
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
	/// Gets the end location of the failure,
	/// or the test location
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
}


impl Into<TestOutcome> for TestFail {
	fn into(self) -> TestOutcome { TestOutcome::Fail(self) }
}
impl Into<TestOutcome> for TestSkip {
	fn into(self) -> TestOutcome { TestOutcome::Skip(self) }
}

impl TestOutcome {
	pub fn is_pass(&self) -> bool { self == &TestOutcome::Pass }
	pub fn is_fail(&self) -> bool { matches!(self, TestOutcome::Fail(_)) }
	pub fn is_skip(&self) -> bool { matches!(self, TestOutcome::Skip(_)) }

	/// Creates a TestOutcome from a PanicResult and whether the test should panic,
	/// retreived via [`Test::should_panic`]
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
