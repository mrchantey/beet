use crate::prelude::*;



#[deprecated = "use custom runner"]
pub fn run_libtest_pretty(tests: &[&test::TestDescAndFn]) {
	return test_main_with_filenames(tests);
}

/// Pretty much run libtest as-is but with pretty filenames for unit tests.
fn test_main_with_filenames(tests: &[&test::TestDescAndFn]) {
	let tests = apply_filenames(tests);
	let tests = tests.iter().collect::<Vec<_>>();
	println!("\n{}\n", RunnerLogger::SWEET_AS);
	test::test_main_static(&tests);
}

fn apply_filenames(tests: &[&test::TestDescAndFn]) -> Vec<test::TestDescAndFn> {
	tests
		.into_iter()
		.map(|test| {
			let mut test = TestDescAndFnExt::clone(test);
			test.desc.name = test::DynTestName(format!(
				"{} - {}",
				test.desc.source_file,
				TestDescExt::short_name(&test.desc)
			));
			// test::StaticTestName(test.desc.source_file);
			test
		})
		.collect()
}
