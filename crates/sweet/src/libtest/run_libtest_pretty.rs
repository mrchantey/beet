use crate::prelude::*;


/// Run libtest with pretty filenames for unit tests.
pub fn run_libtest_pretty(tests: &[&test::TestDescAndFn]) {
	return test_main_with_filenames(tests);
}

/// Pretty much run libtest as-is but with pretty filenames for unit tests.
fn test_main_with_filenames(tests: &[&test::TestDescAndFn]) {
	let tests = apply_filenames(tests);
	let tests = tests.iter().collect::<Vec<_>>();
	test::test_main_static(&tests);
}

fn apply_filenames(tests: &[&test::TestDescAndFn]) -> Vec<test::TestDescAndFn> {
	tests
		.into_iter()
		.map(|test| {
			let mut test = test_ext::clone_static(test);
			test.desc.name = test::DynTestName(test.desc.short_file_and_name());
			test
		})
		.collect()
}
