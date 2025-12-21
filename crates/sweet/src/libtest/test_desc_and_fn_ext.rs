use crate::prelude::*;
use test::TestDescAndFn;
use test::TestFn;

pub fn new(
	name: &str,
	file: &'static str,
	func: impl 'static + Send + FnOnce() -> Result<(), String>,
) -> TestDescAndFn {
	TestDescAndFn {
		desc: test_desc_ext::new(name, file),
		testfn: TestFn::DynTestFn(Box::new(func)),
	}
}


/// copied from https://github.com/rust-lang/rust/blob/a25032cf444eeba7652ce5165a2be450430890ba/library/test/src/lib.rs#L223
/// Clones static values for putting into a dynamic vector, which test_main()
/// needs to hand out ownership of tests to parallel test runners.
///
/// This will panic when fed any dynamic tests, because they cannot be cloned.
pub fn clone(test: &TestDescAndFn) -> TestDescAndFn {
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
