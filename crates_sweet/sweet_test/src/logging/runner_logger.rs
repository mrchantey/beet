use crate::prelude::*;
use colorize::*;
use std::sync::Arc;
use std::time::Duration;
use test::TestDescAndFn;
use web_time::Instant;
use anyhow::Result;

pub struct RunnerLogger {
	start_time: Instant,
	case_logger: CaseLoggerEnum,
	config: Arc<TestRunnerConfig>,
	cases: Vec<TestDescAndResult>,
}


fn clear() {
	#[cfg(target_arch = "wasm32")]
	web_sys::console::clear();
	#[cfg(not(target_arch = "wasm32"))]
	sweet_fs::prelude::terminal::clear().unwrap();
}

impl RunnerLogger {
	pub const SWEET_AS: &'static str = "🤘 sweet as 🤘";

	pub fn start(
		config: Arc<TestRunnerConfig>,
		tests: &[&TestDescAndFn],
	) -> Self {
		let case_logger = CaseLoggerEnum::new(config.clone(), tests);
		if !config.quiet && config.watch {
			clear();
		}
		if !config.quiet {
			sweet_utils::log!("\n{}\n\n{config}", Self::SWEET_AS)
		}

		Self {
			start_time: Instant::now(),
			cases: Vec::new(),
			case_logger,
			config,
		}
	}
	pub fn on_result(&mut self, mut result: TestDescAndResult) -> Result<()> {
		if !self.config.quiet {
			self.case_logger.on_result(&mut result)?;
		}
		self.cases.push(result);
		Ok(())
	}

	/// Finalize outputs and exit with code 1 if failed
	pub fn end(mut self) {
		let result_count = ResultCount::from_case_results(&self.cases);

		if !self.config.quiet {
			sweet_utils::log_val(&self.case_results(&result_count));
		}
		self.on_results_printed();
		if !self.config.watch && !result_count.succeeded() {
			#[cfg(target_arch = "wasm32")]
			js_runtime::exit(1);
			#[cfg(not(target_arch = "wasm32"))]
			std::process::exit(1);
		}
	}

	fn on_results_printed(&mut self) {}
	fn case_results(&mut self, results: &ResultCount) -> String {
		let mut post_run = String::from("\n");

		if results.is_empty() {
			post_run += "No Tests Found\n".red().as_str();
			return post_run;
		} else if results.succeeded() {
			post_run +=
				"All tests passed\n".bold().cyan().underlined().as_str();
		}

		if let Some(case_logger_end_str) = self.case_logger.end_str() {
			post_run += case_logger_end_str.as_str();
			post_run.push('\n');
		}

		// post_run += suites.pretty_print("Suites:\t\t").as_str();
		// post_run.push('\n');
		post_run += results.pretty_print("Tests").as_str();
		post_run.push('\n');
		post_run += print_time(self.start_time.elapsed()).as_str();
		post_run
	}
}

fn print_time(duration: Duration) -> String {
	let millis = duration.as_millis();
	let prefix = "Time: \t\t".bold();
	if millis < 100 {
		format!("{}{} ms\n\n", prefix, millis)
	} else {
		format!("{}{:.2} s\n\n", prefix, millis as f32 * 0.001)
	}
}
