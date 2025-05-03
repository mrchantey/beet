use crate::prelude::*;
use colorize::*;

#[derive(Debug, Default)]
pub struct ResultCount {
	pub total: usize,
	pub failed: usize,
	pub skipped: usize,
}


impl ResultCount {
	pub fn from_file_results<'a>(
		results: impl Iterator<Item = &'a FileResult>,
	) -> Self {
		let mut count = Self::default();
		for result in results {
			count.append_result(&result.status());
		}
		count
	}
	pub fn from_case_results(results: &Vec<TestDescAndResult>) -> Self {
		let mut count = Self::default();
		for result in results {
			count.append_result(&result.result);
		}
		count
	}

	pub fn append_result(&mut self, result: &TestResult) {
		self.total += 1;
		match result {
			TestResult::Pass => {}
			TestResult::Fail(_) => self.failed += 1,
			TestResult::Ignore(_) => self.skipped += 1,
		}
	}

	pub fn is_empty(&self) -> bool { self.total == 0 }
	pub fn succeeded(&self) -> bool { self.failed == 0 }


	pub fn pretty_print(&self, prefix: &'static str) -> String {
		let ResultCount {
			total,
			failed,
			skipped,
		} = self;
		let passed = total - failed - skipped;
		let mut summaries: Vec<&str> = Vec::new();
		let passed_str = format!("{passed} passed").bold().green();
		let skipped_str = format!("{skipped} skipped").bold().yellow();
		let failed_str = format!("{failed} failed").bold().red();
		let total_str = if passed == *total {
			format!("{total} total")
		} else {
			format!("{passed} of {total} total")
		};
		if *failed > 0 {
			summaries.push(&failed_str);
		}
		if *skipped > 0 {
			summaries.push(&skipped_str);
		}
		summaries.push(&passed_str);
		summaries.push(&total_str);
		format!("{}:\t\t{}", prefix.bold(), summaries.join(", "))
	}
}
