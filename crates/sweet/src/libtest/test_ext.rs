use crate::prelude::*;
use test::TestDescAndFn;
use test::TestFn;

/// Uses this file and an incrementing name: `test1`, `test2` etc
pub fn anon(
	func: impl 'static + Send + FnOnce() -> Result<(), String>,
) -> TestDescAndFn {
	static COUNTER: std::sync::atomic::AtomicUsize =
		std::sync::atomic::AtomicUsize::new(1);
	let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
	let name = format!("test{}", id);

	TestDescAndFn {
		desc: test_ext::new_desc(&name, file!()),
		testfn: TestFn::DynTestFn(Box::new(func)),
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
		_ => panic!("non-static tests passed to test::test_main_static"),
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
