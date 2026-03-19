//! Pretty-printing test runner that delegates to libtest.
//!
//! Only available with the `custom_test_framework` feature since it
//! requires the nightly `test` crate.

#[cfg(feature = "custom_test_framework")]
use crate::prelude::*;
#[cfg(feature = "custom_test_framework")]
use crate::testing::utils::*;


/// Run libtest with pretty filenames for unit tests.
///
/// Only available with the `custom_test_framework` feature.
#[cfg(feature = "custom_test_framework")]
pub fn test_runner(tests: &[&test::TestDescAndFn]) {
	let tests = apply_filenames(tests);
	let borrowed = tests.iter().collect::<Vec<_>>();
	test::test_main_static(&borrowed);
}


/// Clones a nightly test and replaces its name with a short file + name format.
#[cfg(feature = "custom_test_framework")]
fn apply_filenames(tests: &[&test::TestDescAndFn]) -> Vec<test::TestDescAndFn> {
	tests
		.iter()
		.map(|test| {
			// Convert to beet's TestDesc to use the short_file_and_name helper
			let beet_desc: crate::testing::runner::TestDesc =
				test.desc.clone().into();
			let short_name = beet_desc.short_file_and_name();

			// Clone the nightly test staying in the test:: type system
			let testfn = match test.testfn {
				test::TestFn::StaticTestFn(func) => {
					test::TestFn::StaticTestFn(func)
				}
				test::TestFn::StaticBenchFn(func) => {
					test::TestFn::StaticBenchFn(func)
				}
				_ => panic!("non-static tests cannot be cloned"),
			};

			test::TestDescAndFn {
				desc: test::TestDesc {
					name: test::DynTestName(short_name),
					..test.desc.clone()
				},
				testfn,
			}
		})
		.collect()
}
