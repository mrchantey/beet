use crate::prelude::*;
use crate::testing::utils::*;


/// Run libtest with pretty filenames for unit tests.
pub fn test_runner(tests: &[&test::TestDescAndFn]) {
	let tests = apply_filenames(tests);
	let borrowed = tests.iter().collect::<Vec<_>>();
	test::test_main_static(&borrowed);
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
