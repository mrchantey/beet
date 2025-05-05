use crate::prelude::*;
use anyhow::Result;
use flume::Sender;
use test::TestDescAndFn;


/// The first step of the runner is to:
/// - Clone the tests
/// - Sort by source file unless `--shuffle`
/// - Filter out ignored unless `--ignored` or `--include-ignored`
pub fn collect_runnable_tests(
	config: &TestRunnerConfig,
	result_tx: &Sender<TestDescAndResult>,
	tests: &[&TestDescAndFn],
) -> Result<Vec<TestDescAndFn>> {
	let tests = tests
		.into_iter()
		.filter_map(|test| {
			if let Some(ignore_msg) = config.should_not_run(test) {
				result_tx
					.send(TestDescAndResult::new(
						test.desc.clone(),
						TestResult::Ignore(Some(ignore_msg.to_string())),
					))
					.expect("channel was dropped");
				None
			} else {
				Some(TestDescAndFnExt::clone(test))
			}
		})
		.collect::<Vec<_>>();
	// already sorted
	// tests.sort_by(|a, b| a.desc.source_file.cmp(&b.desc.source_file));
	Ok(tests)
}


pub trait TestRunner {
	fn collect_and_run(
		config: &TestRunnerConfig,
		future_tx: Sender<TestDescAndFuture>,
		result_tx: Sender<TestDescAndResult>,
		tests: &[&TestDescAndFn],
	) -> Result<()> {
		let tests = collect_runnable_tests(config, &result_tx, tests)?;
		Self::run(config, future_tx, result_tx, tests)?;
		Ok(())
	}


	fn run(
		config: &TestRunnerConfig,
		future_tx: Sender<TestDescAndFuture>,
		result_tx: Sender<TestDescAndResult>,
		tests: Vec<TestDescAndFn>,
	) -> Result<()>;


	// fn collect_and_run(tests: &[&TestDescAndFn]) -> Vec<Self> {
	// 	let mut suites = HashMap::new();
	// 	for test in tests.iter() {
	// 		let suite = suites
	// 			.entry(test.desc.source_file)
	// 			.or_insert_with(|| TestSuite::new(test.desc.source_file));
	// 		suite.tests.push(TestDescAndFnExt::clone(test));
	// 	}
}


// pub struct TestRunner {}


// impl TestRunner {
// 	pub fn run() {
// 		// assert
// 	}
// }
