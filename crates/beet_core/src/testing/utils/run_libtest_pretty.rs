//! Run libtest with pretty filenames for unit tests.
//!
//! Only available on the nightly `custom_test_frameworks` path, since it
//! depends on `test::test_main_static`.

use crate::prelude::*;
use crate::testing::utils::*;

/// Run libtest with pretty filenames for unit tests.
pub fn test_runner(tests: &[&test::TestDescAndFn]) {
	let tests = apply_filenames(tests);
	let borrowed = tests.iter().collect::<Vec<_>>();
	test::test_main_static(&borrowed);
}

fn clone_static(test: &test::TestDescAndFn) -> test::TestDescAndFn {
	match test.testfn {
		test::TestFn::StaticTestFn(f) => test::TestDescAndFn {
			testfn: test::TestFn::StaticTestFn(f),
			desc: test.desc.clone(),
		},
		test::TestFn::StaticBenchFn(f) => test::TestDescAndFn {
			testfn: test::TestFn::StaticBenchFn(f),
			desc: test.desc.clone(),
		},
		_ => panic!("non-static tests cannot be cloned"),
	}
}

fn apply_filenames(
	tests: &[&test::TestDescAndFn],
) -> Vec<test::TestDescAndFn> {
	tests
		.iter()
		.map(|test| {
			let mut test = clone_static(test);
			let desc: TestDesc = (&test.desc).into();
			test.desc.name = test::DynTestName(desc.short_file_and_name());
			test
		})
		.collect()
}
