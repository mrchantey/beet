use crate::prelude::*;
use test::TestDescAndFn;


/// Entry point for the sweet test runner
/// ## Panics
/// Panics if dynamic tests or benches are passed in, see [`test_desc_and_fn_ext::clone`]
pub fn run_static(tests: &[&TestDescAndFn]) {
	let tests = tests
		.iter()
		.map(|test| test_desc_and_fn_ext::clone(test))
		.collect();
	run(tests);
}
/// Runs an owned set of tests.
pub fn run(tests: Vec<TestDescAndFn>) {}
