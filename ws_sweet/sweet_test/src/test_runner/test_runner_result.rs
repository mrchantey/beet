use crate::prelude::*;

#[derive(Debug)]
pub struct TestRunnerResult {
	pub suite_results: Vec<SuiteResult>,
	pub suites: ResultCount,
	pub cases: ResultCount, //TODO probably newtype this
}

impl Into<TestRunnerResult> for Vec<SuiteResult> {
	fn into(self) -> TestRunnerResult {
		TestRunnerResult::from_suite_results(self)
	}
}

impl TestRunnerResult {
	/// Common finalization for both native and wasm runners
	// pub fn finalize(
	// 	config: TestRunnerConfig,
	// 	logger: impl RunnerLogger,
	// 	mut suite_results: Vec<SuiteResult>,
	// 	mut async_suite_outputs: Vec<SuiteOutput>,
	// 	async_test_outputs: Vec<(TestDesc, TestOutput)>,
	// ) {
	// 	SuiteOutput::extend_test_outputs(
	// 		&mut async_suite_outputs,
	// 		async_test_outputs,
	// 	);
	// 	let async_results =
	// 		SuiteOutput::finalize_all(&config, async_suite_outputs);
	// 	suite_results.extend(async_results);

	// 	Self::from_suite_results(suite_results).end(&config, logger);
	// }





	pub fn did_fail(&self) -> bool { self.cases.failed > 0 }

	pub fn from_suite_results(suite_results: Vec<SuiteResult>) -> Self {
		let mut suites = ResultCount::default();
		let cases = suite_results.iter().fold(
			ResultCount::default(),
			|mut acc, item| {
				acc.total += item.num_tests;
				acc.failed += item.failed.len();
				acc.skipped += item.num_ignored;

				suites.total += 1;
				if item.failed.len() > 0 {
					suites.failed += 1;
				}

				acc
			},
		);
		TestRunnerResult {
			suite_results,
			suites,
			cases,
		}
	}
}
