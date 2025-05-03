use crate::prelude::*;
use anyhow::Result;
use colorize::AnsiColor;
use std::collections::HashMap;
use test::TestDescAndFn;


#[derive(Debug)]
pub struct FileResult {
	file: &'static str,
	results: Vec<TestResult>,
	total: usize,
}
impl FileResult {
	pub fn new(file: &'static str) -> Self {
		Self {
			file,
			results: Vec::new(),
			total: 0,
		}
	}

	pub fn increment_total(&mut self) { self.total += 1; }


	/// - If **any** error, the status is [TestResult::Fail]
	/// - If **all** ignored, the status is [TestResult::Ignore]
	/// - Otherwise the status is [TestResult::Pass]
	pub fn status(&self) -> TestResult {
		let mut failed: Vec<&str> = Vec::new();
		let mut ignored = 0;
		for result in &self.results {
			match result {
				TestResult::Pass => {}
				TestResult::Fail(msg) => failed.push(msg.as_str()),
				TestResult::Ignore(_) => ignored += 1,
			}
		}
		if !failed.is_empty() {
			TestResult::Fail(failed.join("\n"))
		} else if ignored == self.total {
			TestResult::Ignore(None)
		} else {
			TestResult::Pass
		}
	}

	/// pushes the result and logs if complete
	pub fn push_and_maybe_log(&mut self, result: TestResult) {
		self.results.push(result);
		if self.results.len() != self.total {
			return;
		}
		
		// its finished so log if not skipped
		let status = self.status();
		if let TestResult::Ignore(_) = status {
			return;
		}
		let prefix = status.status_prefix();
		let file = pretty_file_path(&self.file);
		sweet_utils::log!("{} {}{}", prefix, file, status.message());
	}
}


#[derive(Default)]
pub struct FileLogger {
	test_counters: HashMap<&'static str, FileResult>,
}

impl FileLogger {
	pub fn new(tests: &[&TestDescAndFn]) -> Self {
		let mut test_counters: HashMap<&'static str, FileResult> =
			HashMap::new();
		for test in tests {
			test_counters
				.entry(test.desc.source_file)
				.or_insert(FileResult::new(test.desc.source_file))
				.increment_total();
		}
		Self { test_counters }
	}
}

fn prepend_failing_test_name(result: &mut TestDescAndResult) {
	if let TestResult::Fail(val) = &mut result.result {
		let file_stem = std::path::Path::new(result.desc.source_file)
			.file_stem()
			.unwrap_or_default()
			.to_str()
			.unwrap_or_default();

		let test_name = TestDescExt::short_name(&result.desc);

		let next = format!("\n\t• {file_stem} > {test_name} \n\n").red();
		*val = format!("{next}{val}");
	}
}


impl CaseLogger for FileLogger {
	fn on_result(&mut self, result: &mut TestDescAndResult) -> Result<()> {
		prepend_failing_test_name(result);


		let Some(counter) = self.test_counters.get_mut(result.desc.source_file)
		else {
			anyhow::bail!(
				"No test counter found for file: {}",
				result.desc.source_file
			)
		};
		counter.push_and_maybe_log(result.result.clone());
		Ok(())
	}

	fn end_str(&mut self) -> Option<String> {
		let results =
			ResultCount::from_file_results(self.test_counters.values());
		Some(results.pretty_print("Files"))
	}
}
