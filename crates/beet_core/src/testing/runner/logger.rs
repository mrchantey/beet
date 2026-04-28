use itertools::Itertools;

use crate::prelude::*;
use crate::testing::runner::*;
use crate::testing::utils::*;



#[allow(unused)]
pub(super) fn log_suite_running(
	requests: Populated<(Entity, &TestRunnerConfig), Added<TestRunnerConfig>>,
) -> Result {
	for (_entity, config) in requests {
		if config.watch {
			terminal_ext::clear().ok();
		}

		if config.quiet {
			continue;
		}
		let _guard = paint_ext::SetPaintEnabledTemp::new(!config.no_color);

		let mut out = Vec::new();

		out.push("🌱 beet test 🌱".to_string());


		crate::cross_log!("\n{}\n", out.join("\n"));
	}
	Ok(())
}

/// Collects test outcomes once all tests have finished running
pub(super) fn log_case_running(
	requests: Populated<(&TestRunnerConfig, &Children), With<TestRunnerConfig>>,
	just_started: Populated<&Test, (Added<Test>, Without<TestOutcome>)>,
) -> Result {
	for (config, children) in requests {
		if !config.log_runs {
			continue;
		}
		let _guard = paint_ext::SetPaintEnabledTemp::new(!config.no_color);


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
	requests: Populated<
		(Entity, &TestRunnerConfig, &Children),
		With<TestRunnerConfig>,
	>,
	just_finished: Populated<(&Test, &TestOutcome), Added<TestOutcome>>,
) -> Result {
	for (_entity, config, children) in requests {
		if config.quiet || config.no_incremental || !config.log_cases {
			continue;
		}
		let _guard = paint_ext::SetPaintEnabledTemp::new(!config.no_color);

		let just_finished = children
			.iter()
			.filter_map(|child| just_finished.get(child).ok())
			.collect::<Vec<_>>();

		for (test, outcome) in &just_finished {
			if outcome.is_skip() && !config.log_skipped {
				continue;
			}
			test_heading_log(&outcome.ansi_str(), &test).xprint_display();
		}
	}
	Ok(())
}



fn log_case_runs(test: &Test) -> String {
	let prefix = paint_ext::bg_yellow_black_bold(" RUNS ");
	test_heading_log(&prefix, test)
}

/// Collects test outcomes once all tests have finished running
pub(super) fn log_file_outcomes(
	requests: Populated<
		(Entity, &TestRunnerConfig, &Children),
		With<TestRunnerConfig>,
	>,
	_trigger: Populated<(), Added<TestOutcome>>,
	running: Query<&Test, Without<TestOutcome>>,
	finished: Query<(&Test, &TestOutcome)>,
) -> Result {
	for (_entity, config, children) in requests {
		if config.quiet || config.no_incremental || config.log_cases {
			continue;
		}
		let _guard = paint_ext::SetPaintEnabledTemp::new(!config.no_color);

		let running = children
			.iter()
			.filter_map(|child| running.get(child).ok())
			.map(|test| test.source_file)
			.collect::<HashSet<_>>();

		let finished = children
			.iter()
			.filter_map(|child| finished.get(child).ok())
			.fold(HashMap::new(), |mut map, (test, outcome)| {
				if !running.contains(test.source_file) {
					map.entry(test.short_file())
						.or_insert_with(Vec::new)
						.push((test, outcome));
				}
				map
			});

		for (short_file, finished) in
			finished.into_iter().sorted_by_key(|a| a.0)
		{
			use TestOutcome::*;
			if finished.iter().any(|(_test, outcome)| outcome.is_fail()) {
				// if any failed, fall back to individual logging
				for (test, outcome) in finished {
					test_heading_log(&outcome.ansi_str(), &test)
						.xprint_display();
				}
			} else {
				let outcome =
					finished.iter().fold(Pass, |acc, (_, outcome)| {
						match (acc, outcome) {
							(Pass, Pass) => Pass,
							(_, Fail(_)) => {
								Fail(outcome.as_fail().unwrap().clone())
							}
							(acc, Skip(reason)) => {
								if acc.is_fail() {
									acc
								} else {
									Skip(reason.clone())
								}
							}
							(acc, _) => acc,
						}
					});
				if !outcome.is_skip() {
					format!(
						"{} {}",
						outcome.ansi_str(),
						short_file.to_string()
					)
					.xprint_display();
				}
			}
		}
	}
	Ok(())
}

fn test_heading_log(prefix: &str, test: &Test) -> String {
	format!("{} {}", prefix, test.short_file_and_name())
}


pub(super) fn log_suite_outcome(
	requests: Populated<
		(&TestRunnerConfig, &SuiteOutcome, &Children),
		Added<SuiteOutcome>,
	>,
	tests: Query<(&Test, &TestOutcome)>,
) -> Result {
	for (config, outcome, children) in requests {
		if config.quiet {
			continue;
		}
		let _guard = paint_ext::SetPaintEnabledTemp::new(!config.no_color);

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
		out.push(run_stats(outcome, config));
		crate::cross_log!("\n{}\n", out.join("\n"));
	}

	Ok(())
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
	if outcome.num_ran() == 0 {
		stats.push(paint_ext::yellow_bold(format!("no tests ran")));
	}

	stats.join(", ")
}

fn run_stats(outcome: &SuiteOutcome, config: &TestRunnerConfig) -> String {
	let duration = config.started().elapsed();
	let time = time_ext::pretty_print_duration(duration);
	let time = paint_ext::blue_bold(time);
	let test_stats = test_stats(outcome);
	format!("{} in {}", test_stats, time)
}

fn failed_file_context(test: &Test, outcome: &TestFail) -> Result<String> {
	const LINE_CONTEXT_SIZE: usize = 2;
	const TAB_SPACES: usize = 2;

	let path = outcome.path(test).into_abs();
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
	let path = paint_ext::cyan(outcome.path(test).to_string());
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
			TestRunnerConfig::from_cli_str("--quiet"),
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
