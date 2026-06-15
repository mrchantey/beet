//! Beet-owned mirror of the nightly `test` crate descriptor types.
//!
//! These types let the runner work on **stable** Rust. The nightly
//! `custom_test_frameworks` path converts `test::*` into these via the
//! `#[cfg(feature = "custom_test_frameworks")]` shims at the bottom.

// `alloc` (not `std`) so the descriptor types are no_std-ready (eg the esp32
// firmware test build); these are identical to the `std` types on std.
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Mirror of `test::TestName`.
#[derive(Debug, Clone)]
pub enum TestName {
	/// A `&'static str` name, known at compile time.
	Static(&'static str),
	/// A dynamically constructed name.
	Dyn(String),
}

impl TestName {
	/// Returns the name as a string slice.
	pub fn as_slice(&self) -> &str {
		match self {
			Self::Static(s) => s,
			Self::Dyn(s) => s.as_str(),
		}
	}
}

impl core::fmt::Display for TestName {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(self.as_slice())
	}
}

/// Mirror of `test::options::ShouldPanic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShouldPanic {
	/// The test should not panic.
	No,
	/// The test should panic.
	Yes,
	/// The test should panic with a message containing this string.
	YesWithMessage(&'static str),
}

/// Mirror of `test::types::TestType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestType {
	/// Unit test, ie in a `src` file.
	UnitTest,
	/// Integration test, ie in a `tests` directory.
	IntegrationTest,
	/// Doctest.
	DocTest,
	/// Unknown origin.
	Unknown,
}

/// Mirror of `test::TestDesc`.
#[derive(Debug, Clone)]
pub struct TestDesc {
	/// Fully qualified test name.
	pub name: TestName,
	/// Whether the test is ignored.
	pub ignore: bool,
	/// Optional `#[ignore = "message"]` message.
	pub ignore_message: Option<&'static str>,
	/// Source file the test was declared in.
	pub source_file: &'static str,
	/// Start line of the test function.
	pub start_line: usize,
	/// Start column of the test function.
	pub start_col: usize,
	/// End line of the test function.
	pub end_line: usize,
	/// End column of the test function.
	pub end_col: usize,
	/// Whether the test is a `compile_fail` doctest.
	pub compile_fail: bool,
	/// Whether the test should not be run.
	pub no_run: bool,
	/// `#[should_panic]` configuration.
	pub should_panic: ShouldPanic,
	/// The kind of test.
	pub test_type: TestType,
}

/// Mirror of the subset of `test::TestFn` beet uses.
pub enum TestFn {
	/// A static test function pointer.
	StaticTestFn(fn() -> Result<(), String>),
	/// A boxed, dynamically dispatched test function.
	DynTestFn(Box<dyn 'static + Send + FnOnce() -> Result<(), String>>),
}

impl core::fmt::Debug for TestFn {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::StaticTestFn(_) => f.write_str("StaticTestFn(..)"),
			Self::DynTestFn(_) => f.write_str("DynTestFn(..)"),
		}
	}
}

/// Mirror of `test::TestDescAndFn`.
#[derive(Debug)]
pub struct TestDescAndFn {
	/// Description and metadata for the test.
	pub desc: TestDesc,
	/// The test function.
	pub testfn: TestFn,
}

/// The error string a `#[beet_core::test]` run yields on failure: an `alloc`
/// [`String`].
///
/// The `#[beet_core::test]` macro names the generated runner fn's return type
/// through this `beet_core::testing` re-export rather than `alloc`/`std`
/// directly: integration tests and downstream crates already `use
/// beet_core::testing;` (so `crate::testing::*` / `beet::testing::*` resolves),
/// but do not import `beet_core::_alloc`, and `std::string::String` does not
/// exist on the no_std device.
pub type TestError = String;

/// Converts a test's return value into the runner's `Result<(), String>`.
///
/// Mirrors libtest's use of `std::process::Termination` for
/// `#[test] fn() -> Result<_, _>`.
pub trait IntoTestResult {
	/// Performs the conversion.
	fn into_test_result(self) -> Result<(), String>;
}

impl IntoTestResult for () {
	fn into_test_result(self) -> Result<(), String> { Ok(()) }
}

impl<E: core::fmt::Debug> IntoTestResult for Result<(), E> {
	fn into_test_result(self) -> Result<(), String> {
		self.map_err(|e| format!("{e:?}"))
	}
}

/// A test case registered via [`inventory`] (the stable-Rust path).
///
/// Each `#[beet::test]` (without the `custom_test_frameworks` feature)
/// submits one of these. Location/name are captured via `file!()`,
/// `line!()`, `column!()` and `module_path!()` tokens at the call site.
pub struct BeetTestCase {
	/// Fully qualified test name (`module_path!()::ident`).
	pub name: &'static str,
	/// Source file (`file!()`).
	pub source_file: &'static str,
	/// Declaration line (`line!()`).
	pub line: u32,
	/// Declaration column (`column!()`).
	pub col: u32,
	/// `#[should_panic]` configuration.
	pub should_panic: ShouldPanic,
	/// Whether the test is `#[ignore]`d.
	pub ignore: bool,
	/// Optional `#[ignore = "message"]` message.
	pub ignore_message: Option<&'static str>,
	/// The test runner function. For async tests this registers the
	/// future via `register_test` then returns `Ok(())`.
	pub run: fn() -> Result<(), String>,
}

impl BeetTestCase {
	/// Const constructor, called from the `inventory::submit!` the macro emits.
	pub const fn new(
		name: &'static str,
		source_file: &'static str,
		line: u32,
		col: u32,
		should_panic: ShouldPanic,
		ignore: bool,
		ignore_message: Option<&'static str>,
		run: fn() -> Result<(), String>,
	) -> Self {
		Self {
			name,
			source_file,
			line,
			col,
			should_panic,
			ignore,
			ignore_message,
			run,
		}
	}

	/// Converts this inventory case into a [`TestDescAndFn`].
	pub fn to_desc_and_fn(&self) -> TestDescAndFn {
		TestDescAndFn {
			desc: TestDesc {
				name: TestName::Static(self.name),
				ignore: self.ignore,
				ignore_message: self.ignore_message,
				source_file: self.source_file,
				start_line: self.line as usize,
				start_col: self.col as usize,
				end_line: self.line as usize,
				end_col: self.col as usize,
				compile_fail: false,
				no_run: false,
				should_panic: self.should_panic,
				test_type: TestType::UnitTest,
			},
			testfn: TestFn::StaticTestFn(self.run),
		}
	}
}

#[cfg(feature = "testing")]
inventory::collect!(BeetTestCase);

/// Collects all [`BeetTestCase`]s registered via [`inventory`].
#[cfg(feature = "testing")]
pub fn inventory_tests() -> Vec<TestDescAndFn> {
	inventory::iter::<BeetTestCase>
		.into_iter()
		.map(BeetTestCase::to_desc_and_fn)
		.collect()
}

/// The `linkme` distributed slice every embedded `#[beet_core::test]` registers
/// into. `inventory`'s life-before-main constructors never run on bare metal
/// (xtensa-lx-rt does not walk `.init_array`), so the embedded build collects
/// cases from a dedicated linker section instead. Declared here, populated by
/// the `submit!` the `#[beet_core::test]` macro emits under `testing_embedded`.
#[cfg(feature = "testing_embedded")]
#[linkme::distributed_slice]
pub static BEET_TESTS: [BeetTestCase];

/// Collects all [`BeetTestCase`]s registered via the [`BEET_TESTS`] `linkme`
/// slice. The embedded mirror of [`inventory_tests`].
#[cfg(feature = "testing_embedded")]
pub fn embedded_tests() -> Vec<TestDescAndFn> {
	BEET_TESTS.iter().map(BeetTestCase::to_desc_and_fn).collect()
}

#[cfg(feature = "custom_test_frameworks")]
mod libtest_conv {
	use super::*;

	impl From<&test::TestName> for TestName {
		fn from(name: &test::TestName) -> Self {
			match name {
				test::TestName::StaticTestName(s) => TestName::Static(s),
				other => TestName::Dyn(other.as_slice().to_string()),
			}
		}
	}

	impl From<test::ShouldPanic> for ShouldPanic {
		fn from(sp: test::ShouldPanic) -> Self {
			match sp {
				test::ShouldPanic::No => ShouldPanic::No,
				test::ShouldPanic::Yes => ShouldPanic::Yes,
				test::ShouldPanic::YesWithMessage(m) => {
					ShouldPanic::YesWithMessage(m)
				}
			}
		}
	}

	impl From<test::TestType> for TestType {
		fn from(t: test::TestType) -> Self {
			match t {
				test::TestType::UnitTest => TestType::UnitTest,
				test::TestType::IntegrationTest => TestType::IntegrationTest,
				test::TestType::DocTest => TestType::DocTest,
				test::TestType::Unknown => TestType::Unknown,
			}
		}
	}

	impl From<&test::TestDesc> for TestDesc {
		fn from(d: &test::TestDesc) -> Self {
			TestDesc {
				name: (&d.name).into(),
				ignore: d.ignore,
				ignore_message: d.ignore_message,
				source_file: d.source_file,
				start_line: d.start_line,
				start_col: d.start_col,
				end_line: d.end_line,
				end_col: d.end_col,
				compile_fail: d.compile_fail,
				no_run: d.no_run,
				should_panic: d.should_panic.into(),
				test_type: d.test_type.into(),
			}
		}
	}

	/// Clones a libtest `&TestDescAndFn` into a beet [`TestDescAndFn`].
	///
	/// # Panics
	/// Panics on non-static test fns, which cannot be cloned (same
	/// constraint as libtest's own `clone_static`).
	impl From<&test::TestDescAndFn> for TestDescAndFn {
		fn from(t: &test::TestDescAndFn) -> Self {
			let testfn = match t.testfn {
				test::TestFn::StaticTestFn(f) => TestFn::StaticTestFn(f),
				_ => panic!(
					"only static test fns are supported via the libtest path"
				),
			};
			TestDescAndFn {
				desc: (&t.desc).into(),
				testfn,
			}
		}
	}
}
