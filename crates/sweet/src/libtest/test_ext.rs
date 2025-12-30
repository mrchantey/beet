use std::panic::Location;

use crate::prelude::*;
use test::TestDescAndFn;
use test::TestFn;

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

#[track_caller]
pub fn new_auto_static(func: fn() -> Result<(), String>) -> TestDescAndFn {
	let desc = new_auto_desc();
	TestDescAndFn {
		desc,
		testfn: TestFn::StaticTestFn(func),
	}
}


#[track_caller]
pub fn new_auto_desc() -> TestDesc {
	let caller = Location::caller();

	let name = caller.file().split('/').last().unwrap_or("").to_string();

	TestDesc {
		name: test::TestName::DynTestName(format!(
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


/// copied from https://github.com/rust-lang/rust/blob/a25032cf444eeba7652ce5165a2be450430890ba/library/test/src/lib.rs#L223
/// Clones static values for putting into a dynamic vector, which test_main()
/// needs to hand out ownership of tests to parallel test runners.
///
/// ## Panics
/// This will panic when fed any dynamic tests, because they cannot be cloned.
pub fn clone_static(test: &TestDescAndFn) -> TestDescAndFn {
	match test.testfn {
		TestFn::StaticTestFn(f) => TestDescAndFn {
			testfn: TestFn::StaticTestFn(f),
			desc: test.desc.clone(),
		},
		TestFn::StaticBenchFn(f) => TestDescAndFn {
			testfn: TestFn::StaticBenchFn(f),
			desc: test.desc.clone(),
		},
		_ => panic!("non-static tests cannot be cloned"),
	}
}


#[deprecated]
pub fn func(test: &TestDescAndFn) -> fn() -> Result<(), String> {
	match test.testfn {
		TestFn::StaticTestFn(func) => func,
		_ => panic!("non-static tests are not supported"),
	}
}

pub fn run(test: TestFn) -> Result<(), String> {
	match test {
		TestFn::StaticTestFn(func) => func(),
		TestFn::DynTestFn(func) => func(),
		_ => panic!("benches not yet supported"),
	}
}

// 	// match test.testfn {
// 	// 	TestFn::StaticTestFn(func) => func(),
// 	// 	TestFn::StaticBenchFn(func) => func(&mut Bencher::()),
// 	// 	_ => panic!("non-static tests are not supported"),
// 	// }
// }
use test::ShouldPanic;
use test::TestDesc;
use test::TestType;

pub fn is_equal_location(a: &TestDesc, b: &TestDesc) -> bool {
	a.source_file == b.source_file && a.start_line == b.start_line
}
/// Creates a new TestDesc with the given name and source file.
/// Other fields are set to sensible default values for a unit test.
pub fn new_desc(name: &str, file: &'static str) -> TestDesc {
	TestDesc {
		name: test::TestName::DynTestName(name.into()),
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




/// The `#[test]` macro replaces results with [useless error messages](https://github.com/rust-lang/rust/blob/a25032cf444eeba7652ce5165a2be450430890ba/library/test/src/lib.rs#L234)
/// so we instead panic and instruct user to use `unwrap`.
/// Also used by async wasm tests, we dont care what the result is, if ya
/// want messages, panic! at the disco
#[deprecated]
pub fn result_to_panic<T, E>(result: Result<T, E>) {
	match result {
		Ok(_) => {}
		Err(_) => {
			panic!(
				"test returned an Err(). Use `unwrap()` instead to see the contents of the error"
			);
		}
	}
}

/// A libtest name is the fully qualified path
/// ie `test_case::backtrace_error::test::result_builder`
/// we want to shorten this to just the last part
pub fn short_name(test: &TestDesc) -> String {
	let path = test.name.to_string();
	path.split("::")
		.last()
		.map(|p| p.to_string())
		.unwrap_or(path)
}
