use crate::prelude::*;
use anyhow::Result;
use colorize::*;
use test::TestDesc;




/// Log each test result as it comes in
#[derive(Debug, Default)]
pub struct SweetCaseLogger;

impl SweetCaseLogger {
	/// Use the PASS syntax to log a test result
	fn log_case_result_sweetly(&self, case: &TestDescAndResult) {
		let status = case.result.status_prefix();

		let file = self.case_pretty_path(&case.desc);
		let name = TestDescExt::short_name(&case.desc).bold();

		let message = case.result.message();

		sweet_utils::log!("{status} {file}\t{name}{message}");
	}

	fn case_pretty_path(&self, desc: &TestDesc) -> String {
		let mut out = pretty_file_path(&desc.source_file);
		out.push_str(&format!(":{}", desc.start_line).faint());


		return out;
		// format!("{file}:{case.line}")
	}
}


impl CaseLogger for SweetCaseLogger {
	fn on_result(&mut self, result: &mut TestDescAndResult) -> Result<()> {
		self.log_case_result_sweetly(&result);
		Ok(())
	}
}
