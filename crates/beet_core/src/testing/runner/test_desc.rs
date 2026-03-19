//! Stable-compatible test descriptor types.
//!
//! These types mirror the nightly `test` crate types but work on stable Rust.
//! On nightly with `custom_test_framework`, conversions from `test::*` types
//! are provided.

use crate::prelude::*;

/// Whether a test is expected to panic.
///
/// Stable equivalent of `test::ShouldPanic`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ShouldPanic {
	/// The test should not panic.
	No,
	/// The test should panic.
	Yes,
	/// The test should panic with a message containing this string.
	YesWithMessage(&'static str),
}

/// The type of test.
///
/// Stable equivalent of `test::TestType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestType {
	/// A unit test.
	UnitTest,
	/// An integration test.
	IntegrationTest,
	/// A doc test.
	DocTest,
	/// An unknown test type.
	Unknown,
}

/// A test name, either static or dynamic.
///
/// Stable equivalent of `test::TestName`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestName {
	/// A statically known test name.
	StaticTestName(&'static str),
	/// A dynamically allocated test name.
	DynTestName(String),
}

impl TestName {
	/// Returns the name as a string slice.
	pub fn as_slice(&self) -> &str {
		match self {
			TestName::StaticTestName(name) => name,
			TestName::DynTestName(name) => name.as_str(),
		}
	}
}

impl core::fmt::Display for TestName {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self.as_slice())
	}
}

/// Description of a single test case.
///
/// Stable equivalent of `test::TestDesc`.
#[derive(Debug, Clone)]
pub struct TestDesc {
	/// The test name.
	pub name: TestName,
	/// Whether the test is ignored.
	pub ignore: bool,
	/// The ignore message, if any.
	pub ignore_message: Option<&'static str>,
	/// The source file path.
	pub source_file: &'static str,
	/// The start line of the test in the source file.
	pub start_line: usize,
	/// The start column of the test in the source file.
	pub start_col: usize,
	/// The end line of the test in the source file.
	pub end_line: usize,
	/// The end column of the test in the source file.
	pub end_col: usize,
	/// Whether the test should fail to compile.
	pub compile_fail: bool,
	/// Whether the test should not be run.
	pub no_run: bool,
	/// Whether the test should panic.
	pub should_panic: ShouldPanic,
	/// The type of test.
	pub test_type: TestType,
}

/// The function type for a test case.
pub enum TestFn {
	/// A static test function.
	StaticTestFn(fn() -> Result<(), String>),
	/// A dynamic test function (boxed closure).
	DynTestFn(Box<dyn 'static + Send + FnOnce() -> Result<(), String>>),
}

impl core::fmt::Debug for TestFn {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			TestFn::StaticTestFn(_) => write!(f, "StaticTestFn(..)"),
			TestFn::DynTestFn(_) => write!(f, "DynTestFn(..)"),
		}
	}
}

/// A complete test case with descriptor and function.
///
/// Stable equivalent of `test::TestDescAndFn`.
pub struct TestDescAndFn {
	/// The test description.
	pub desc: TestDesc,
	/// The test function.
	pub testfn: TestFn,
}

/// A test case registered via `inventory` for the stable test runner.
///
/// Unlike [`TestDescAndFn`], this type stores only a static function pointer,
/// making it `Send + Sync` for safe use with `inventory`.
pub struct InventoryTestEntry {
	/// The test description.
	pub desc: TestDesc,
	/// The test function pointer.
	pub func: fn() -> Result<(), String>,
}

inventory::collect!(InventoryTestEntry);

/// Collects all tests registered via `inventory`.
pub fn collect_inventory_tests() -> Vec<TestDescAndFn> {
	inventory::iter::<InventoryTestEntry>()
		.map(|entry| TestDescAndFn {
			desc: entry.desc.clone(),
			testfn: TestFn::StaticTestFn(entry.func),
		})
		.collect()
}


// ============================================================================
// Conversions from nightly `test::*` types
// ============================================================================

#[cfg(feature = "custom_test_framework")]
impl From<test::ShouldPanic> for ShouldPanic {
	fn from(value: test::ShouldPanic) -> Self {
		match value {
			test::ShouldPanic::No => ShouldPanic::No,
			test::ShouldPanic::Yes => ShouldPanic::Yes,
			test::ShouldPanic::YesWithMessage(msg) => {
				ShouldPanic::YesWithMessage(msg)
			}
		}
	}
}

#[cfg(feature = "custom_test_framework")]
impl From<test::TestType> for TestType {
	fn from(value: test::TestType) -> Self {
		match value {
			test::TestType::UnitTest => TestType::UnitTest,
			test::TestType::IntegrationTest => TestType::IntegrationTest,
			test::TestType::DocTest => TestType::DocTest,
			test::TestType::Unknown => TestType::Unknown,
		}
	}
}

#[cfg(feature = "custom_test_framework")]
impl From<test::TestName> for TestName {
	fn from(value: test::TestName) -> Self {
		match value {
			test::TestName::StaticTestName(name) => {
				TestName::StaticTestName(name)
			}
			test::TestName::DynTestName(name) => TestName::DynTestName(name),
			// AlignedTestName is a variant we can treat as dynamic
			_ => TestName::DynTestName(value.to_string()),
		}
	}
}

#[cfg(feature = "custom_test_framework")]
impl From<test::TestDesc> for TestDesc {
	fn from(value: test::TestDesc) -> Self {
		TestDesc {
			name: value.name.into(),
			ignore: value.ignore,
			ignore_message: value.ignore_message,
			source_file: value.source_file,
			start_line: value.start_line,
			start_col: value.start_col,
			end_line: value.end_line,
			end_col: value.end_col,
			compile_fail: value.compile_fail,
			no_run: value.no_run,
			should_panic: value.should_panic.into(),
			test_type: value.test_type.into(),
		}
	}
}

#[cfg(feature = "custom_test_framework")]
impl From<test::TestFn> for TestFn {
	fn from(value: test::TestFn) -> Self {
		match value {
			test::TestFn::StaticTestFn(func) => TestFn::StaticTestFn(func),
			test::TestFn::DynTestFn(func) => TestFn::DynTestFn(func),
			_ => panic!("bench functions are not supported"),
		}
	}
}

#[cfg(feature = "custom_test_framework")]
impl From<test::TestDescAndFn> for TestDescAndFn {
	fn from(value: test::TestDescAndFn) -> Self {
		TestDescAndFn {
			desc: value.desc.into(),
			testfn: value.testfn.into(),
		}
	}
}

/// Convert a nightly test descriptor reference into beet's stable type.
///
/// Only available with the `custom_test_framework` feature.
#[cfg(feature = "custom_test_framework")]
pub fn from_nightly_ref(test: &test::TestDescAndFn) -> TestDescAndFn {
	// We can only clone static function pointers
	match test.testfn {
		test::TestFn::StaticTestFn(func) => TestDescAndFn {
			desc: test.desc.clone().into(),
			testfn: TestFn::StaticTestFn(func),
		},
		test::TestFn::StaticBenchFn(_) => {
			panic!("bench functions are not supported")
		}
		_ => panic!("non-static tests cannot be cloned from nightly refs"),
	}
}
