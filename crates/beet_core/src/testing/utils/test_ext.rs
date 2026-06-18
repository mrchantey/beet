use crate::prelude::*;
use crate::testing::utils::*;
use core::panic::Location;

/// Uses [`Location::caller`] to propagate test location and name
#[track_caller]
pub fn new_auto(
	func: impl 'static + Send + FnOnce() -> Result<(), String>,
) -> TestDescAndFn {
	let desc = new_auto_desc();
	TestDescAndFn {
		desc,
		testfn: TestFn::DynTestFn(Box::new(func)),
	}
}

/// Creates a test with a static function, using caller location for metadata.
#[track_caller]
pub fn new_auto_static(func: fn() -> Result<(), String>) -> TestDescAndFn {
	let desc = new_auto_desc();
	TestDescAndFn {
		desc,
		testfn: TestFn::StaticTestFn(func),
	}
}

/// Creates a test descriptor using the caller's location for metadata.
#[track_caller]
pub fn new_auto_desc() -> TestDesc {
	let caller = Location::caller();

	let name = caller.file().split('/').last().unwrap_or("").to_string();

	TestDesc {
		name: TestName::Dyn(format!(
			// approximate how a real test name would look
			"libtest::test_ext::{}#{}",
			name,
			caller.line()
		)),
		ignore: false,
		ignore_message: None,
		source_file: caller.file(),
		start_line: caller.line() as usize,
		start_col: caller.column() as usize,
		end_line: caller.line() as usize,
		end_col: caller.column() as usize,
		compile_fail: false,
		no_run: false,
		should_panic: ShouldPanic::No,
		test_type: TestType::UnitTest,
	}
}

/// Creates a test with the given name, file, and function.
pub fn new(
	name: &str,
	file: &'static str,
	func: impl 'static + Send + FnOnce() -> Result<(), String>,
) -> TestDescAndFn {
	TestDescAndFn {
		desc: test_ext::new_desc(name, file),
		testfn: TestFn::DynTestFn(Box::new(func)),
	}
}

/// Clones a static test for handing out ownership of tests to parallel
/// test runners.
///
/// ## Panics
/// This will panic when fed any dynamic tests, because they cannot be cloned.
pub fn clone_static(test: &TestDescAndFn) -> TestDescAndFn {
	match test.testfn {
		TestFn::StaticTestFn(f) => TestDescAndFn {
			testfn: TestFn::StaticTestFn(f),
			desc: test.desc.clone(),
		},
		_ => panic!("non-static tests cannot be cloned"),
	}
}

/// Runs a test function and returns its result.
pub fn run(test: TestFn) -> Result<(), String> {
	match test {
		TestFn::StaticTestFn(func) => func(),
		TestFn::DynTestFn(func) => func(),
	}
}

/// Returns `true` if two test descriptors have the same source location.
pub fn is_equal_location(a: &TestDesc, b: &TestDesc) -> bool {
	a.source_file == b.source_file && a.start_line == b.start_line
}

/// Creates a new TestDesc with the given name and source file.
/// Other fields are set to sensible default values for a unit test.
pub fn new_desc(name: &str, file: &'static str) -> TestDesc {
	TestDesc {
		name: TestName::Dyn(name.into()),
		ignore: false,
		ignore_message: None,
		source_file: file,
		start_line: 0,
		start_col: 0,
		end_line: 0,
		end_col: 0,
		compile_fail: false,
		no_run: false,
		should_panic: ShouldPanic::No,
		test_type: TestType::UnitTest,
	}
}
