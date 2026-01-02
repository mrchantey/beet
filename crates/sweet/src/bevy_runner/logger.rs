use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;


#[derive(Clone, Reflect, Component, Default)]
#[reflect(Default)]
pub(super) struct LoggerParams {
	/// Do not log test outcomes as they complete
	no_incremental: bool,
	/// Log each test name before running it
	log_runs: bool,
	/// Log each skipped test
	log_skipped: bool,
	/// Disable ANSII colored output
	no_color: bool,
	/// Suppress all logger output
	quiet: bool,
}

impl RequestMetaExtractor for LoggerParams {
	fn extract(request: &RequestMeta) -> Result<Self> {
		request.params().parse_reflect()
	}
}

pub(super) fn log_suite_running(
	requests: Populated<(Entity, &RequestMeta), Added<RequestMeta>>,
	mut logger_params: Extractor<LoggerParams>,
	// mut filter_params: Extractor<FilterParams>,
) -> Result {
	for (entity, _req) in requests {
		let logger_params = logger_params.get(entity)?;
		if logger_params.quiet {
			continue;
		}
		let _guard =
			paint_ext::SetPaintEnabledTemp::new(!logger_params.no_color);

		let mut out = Vec::new();

		out.push("🤘 sweet as 🤘".to_string());
		// out.push(format!("Request: {:#?}", req));
		// out.push(format!("Test filter: {:#?}", filter_params));


		beet_core::cross_log!("\n{}\n", out.join("\n"));
	}
	Ok(())
}

/// Collects test outcomes once all tests have finished running
pub(super) fn log_case_running(
	requests: Populated<(Entity, &Children), With<RequestMeta>>,
	just_started: Populated<&Test, (Added<Test>, Without<TestOutcome>)>,
	mut params: Extractor<LoggerParams>,
) -> Result {
	for (entity, children) in requests {
		let params = params.get(entity)?;
		if !params.log_runs {
			continue;
		}
		let _guard = paint_ext::SetPaintEnabledTemp::new(!params.no_color);


		for test in children
			.iter()
			.filter_map(|child| just_started.get(child).ok())
		{
			log_case_runs(&test).xprint_display();
		}
	}
	Ok(())
}

/// Collects test outcomes once all tests have finished running
pub(super) fn log_case_outcomes(
	requests: Populated<(Entity, &Children), With<RequestMeta>>,
	just_finished: Populated<(&Test, &TestOutcome), Added<TestOutcome>>,
	mut params: Extractor<LoggerParams>,
) -> Result {
	for (entity, children) in requests {
		let params = params.get(entity)?;
		if params.quiet || params.no_incremental {
			continue;
		}
		let _guard = paint_ext::SetPaintEnabledTemp::new(!params.no_color);

		let just_finished = children
			.iter()
			.filter_map(|child| just_finished.get(child).ok())
			.collect::<Vec<_>>();

		for (test, outcome) in &just_finished {
			if outcome.is_skip() && !params.log_skipped {
				continue;
			}
			log_case_output(&test, outcome).xprint_display();
		}
	}
	Ok(())
}



fn log_case_runs(test: &Test) -> String {
	let prefix = paint_ext::bg_yellow_black_bold(" RUNS ");
	test_heading_log(&prefix, test)
}


/// Returns the colored or non-colored outcome prefix for the test:
/// - pass: " PASS "
/// - skip: " SKIP "
/// - fail: " FAIL "
fn log_case_output(test: &Test, outcome: &TestOutcome) -> String {
	let prefix = match outcome {
		TestOutcome::Pass => paint_ext::bg_green_black_bold(" PASS "),
		TestOutcome::Skip(_) => paint_ext::bg_yellow_black_bold(" SKIP "),
		TestOutcome::Fail(_) => paint_ext::bg_red_black_bold(" FAIL "),
	};
	test_heading_log(&prefix, test)
}

fn test_heading_log(prefix: &str, test: &Test) -> String {
	format!("{} {}", prefix, test.short_file_and_name())
}


pub(super) fn log_suite_outcome(
	requests: Populated<
		(Entity, &RequestMeta, &SuiteOutcome, &Children),
		Added<SuiteOutcome>,
	>,
	mut params: Extractor<LoggerParams>,
	tests: Query<(&Test, &TestOutcome)>,
) -> Result {
	for (entity, req, outcome, children) in requests {
		let params = params.get(entity)?;
		if params.quiet {
			continue;
		}
		let _guard = paint_ext::SetPaintEnabledTemp::new(!params.no_color);

		let mut out = Vec::new();
		if outcome.num_fail() != 0 {
			for (test, case_outcome) in
				children.iter().filter_map(|child| tests.get(child).ok())
			{
				if let TestOutcome::Fail(fail) = case_outcome {
					out.push(String::new());
					out.push(failed_heading(test, fail));
					out.push(String::new());
					out.push(failed_file_context(test, fail)?);
					out.push(String::new());
					out.push(failed_stacktrace(test, fail));
					out.push(String::new());
				}
			}
		}
		out.push(summary_message(outcome));
		out.push(String::new());
		out.push(run_stats(outcome, req));
		beet_core::cross_log!("\n{}\n", out.join("\n"));
	}

	Ok(())
}

fn summary_message(outcome: &SuiteOutcome) -> String {
	if outcome.num_fail() == 0 && outcome.num_ran() != 0 {
		paint_ext::cyan_bold_underline("All tests passed")
	} else if outcome.num_ran() == 0 {
		paint_ext::red_bold_underline("No tests ran")
	} else {
		paint_ext::red_bold_underline("Some tests failed")
	}
}

fn test_stats(outcome: &SuiteOutcome) -> String {
	let mut stats = Vec::new();
	if outcome.num_fail() > 0 {
		stats.push(paint_ext::red_bold(format!(
			"{} failed",
			outcome.num_fail()
		)));
	}
	if outcome.num_skip() > 0 {
		stats.push(paint_ext::yellow_bold(format!(
			"{} skipped",
			outcome.num_skip()
		)));
	}
	if outcome.num_pass() > 0 {
		stats.push(paint_ext::green_bold(format!(
			"{} passed",
			outcome.num_pass()
		)));
	}
	stats.join(", ")
}

fn run_stats(outcome: &SuiteOutcome, req: &RequestMeta) -> String {
	let duration = req.started().elapsed();
	let time = time_ext::pretty_print_duration(duration);
	let time = paint_ext::blue_bold(time);
	let test_stats = test_stats(outcome);
	format!("{} in {}", test_stats, time)
}

fn failed_file_context(test: &Test, outcome: &TestFail) -> Result<String> {
	const LINE_CONTEXT_SIZE: usize = 2;
	const TAB_SPACES: usize = 2;

	let path = test.path().into_abs();
	let file = fs_ext::read_to_string(path)?;
	let lines = file.split('\n').collect::<Vec<_>>();
	let max_digits = lines.len().to_string().len();

	let start = outcome.start(test);

	let mut output = Vec::new();
	// line number is one-indexed
	let buffer_start_line = usize::max(
		0,
		(start.line() as usize).saturating_sub(LINE_CONTEXT_SIZE + 1),
	);
	let buffer_end_line = usize::min(
		lines.len() - 1,
		(start.line() as usize) + LINE_CONTEXT_SIZE,
	);
	for i in buffer_start_line..buffer_end_line {
		let curr_line_no = i + 1;
		let is_err_line = curr_line_no == start.line() as usize;
		let prefix = if is_err_line {
			paint_ext::red(">")
		} else {
			" ".to_string()
		};

		let buffer = {
			let line_digits = curr_line_no.to_string().len();
			let len = max_digits.saturating_sub(line_digits);
			" ".repeat(len)
		};
		let line_prefix =
			paint_ext::dimmed(format!("{}{}|", curr_line_no, buffer));

		// replace tabs with spaces
		let line_with_spaces = lines[i].replace("\t", &" ".repeat(TAB_SPACES));

		output.push(format!("{} {}{}", prefix, line_prefix, line_with_spaces));
		if is_err_line {
			let empty_line_prefix =
				paint_ext::dimmed(format!("{}|", " ".repeat(2 + max_digits)));
			let col_buffer = " ".repeat(start.col() as usize);
			let up_arrow = paint_ext::red("^");
			output.push(format!(
				"{}{}{}",
				empty_line_prefix, col_buffer, up_arrow
			));
		}
	}

	output.join("\n").xok()
}

fn failed_heading(test: &Test, outcome: &TestFail) -> String {
	let title = paint_ext::red(test.short_file_and_name());
	let reason = fail_reason(outcome);
	format!("{}\n{}", title, reason)
}

fn fail_reason(outcome: &TestFail) -> String {
	match outcome {
		TestFail::Err { message } => {
			let prefix = paint_ext::bold("Returned error:");
			format!("{} {}", prefix, message)
		}
		TestFail::ExpectedPanic { message } => {
			if let Some(message) = message {
				let prefix = paint_ext::bold("Expected panic:");
				format!("{} {}", prefix, message)
			} else {
				paint_ext::bold("Expected panic")
			}
		}
		TestFail::Panic { payload, .. } => {
			if let Some(payload) = payload {
				let prefix = paint_ext::bold("");
				format!("{}\n{}", prefix, payload)
			} else {
				paint_ext::bold("Panic - opaque payload")
			}
		}
		TestFail::Timeout { elapsed } => {
			let prefix = paint_ext::bold("Timed out after:");
			let time = time_ext::pretty_print_duration(*elapsed);
			let time = paint_ext::blue(time);
			format!("{} {}", prefix, time)
		}
	}
}

fn failed_stacktrace(test: &Test, outcome: &TestFail) -> String {
	let prefix = paint_ext::dimmed("at");
	let path = paint_ext::cyan(test.path().to_string());
	let start = outcome.start(test);
	let line_loc =
		paint_ext::dimmed(format!(":{}:{}", start.line(), start.col()));
	format!("{} {}{}", prefix, path, line_loc)
}

#[cfg(test)]
mod tests {
	use super::*;
	use test::TestDescAndFn;

	fn run_tests(tests: Vec<TestDescAndFn>) {
		let mut app = App::new().with_plugins((
			// ensure app exits even with update loop
			MinimalPlugins,
			TestPlugin,
		));
		app.world_mut().spawn((
			Request::from_cli_str("--quiet").unwrap(),
			tests_bundle(tests),
		));
		app.run();
	}

	#[test]
	fn works_sync() {
		// panic!("foo");
		run_tests(vec![
			test_ext::new_auto(|| Ok(())),
			test_ext::new_auto(|| Ok(())).with_should_panic(),
		]);
	}
}
