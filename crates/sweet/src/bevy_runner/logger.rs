use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;
use nu_ansi_term::Color;
use nu_ansi_term::Style;


#[derive(Clone, Reflect, Component)]
pub(super) struct LoggerParams {
	/// Do not log test outcomes as they complete
	no_incremental: bool,
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

pub(super) fn log_initial(
	requests: Populated<(Entity, &RequestMeta), Added<RequestMeta>>,
	mut logger_params: Extractor<LoggerParams>,
	// mut filter_params: Extractor<FilterParams>,
) -> Result {
	for (entity, _req) in requests {
		let logger_params = logger_params.get(entity)?;
		if logger_params.quiet {
			continue;
		}

		let mut out = Vec::new();

		out.push("🤘 sweet as 🤘".to_string());
		// out.push(format!("Request: {:#?}", req));
		// out.push(format!("Test filter: {:#?}", filter_params));


		beet_core::cross_log!("\n{}", out.join("\n"));
	}
	Ok(())
}


/// Collects test outcomes once all tests have finished running
pub(super) fn log_incremental(
	requests: Populated<(Entity, &Children), With<RequestMeta>>,
	just_finished: Populated<(&Test, &TestOutcome), Added<TestOutcome>>,
	mut params: Extractor<LoggerParams>,
) -> Result {
	for (entity, children) in requests {
		let params = params.get(entity)?;
		if params.quiet || params.no_incremental {
			continue;
		}
		let just_finished = children
			.iter()
			.filter_map(|child| just_finished.get(child).ok())
			.collect::<Vec<_>>();

		for (test, outcome) in &just_finished {
			if outcome.is_skip() && !params.log_skipped {
				continue;
			}
			test_output_log(&params, test, outcome).xprint_display();
		}
	}
	Ok(())
}


/// Returns the colored or non-colored outcome prefix for the test:
/// - pass: " PASS "
/// - skip: " SKIP "
/// - fail: " FAIL "
fn outcome_prefix(params: &LoggerParams, outcome: &TestOutcome) -> String {
	match (outcome, params.no_color) {
		(TestOutcome::Pass, false) => Color::Black
			.bold()
			.on(Color::Green)
			.paint(" PASS ")
			.to_string(),
		(TestOutcome::Pass, true) => " PASS ".to_string(),
		(TestOutcome::Skip(_), false) => Color::Black
			.bold()
			.on(Color::Yellow)
			.paint(" SKIP ")
			.to_string(),
		(TestOutcome::Skip(_), true) => " SKIP ".to_string(),
		(TestOutcome::Fail(_), false) => Color::Black
			.bold()
			.on(Color::Red)
			.paint(" FAIL ")
			.to_string(),
		(TestOutcome::Fail(_), true) => " FAIL ".to_string(),
	}
}

fn test_output_log(
	params: &LoggerParams,
	test: &Test,
	outcome: &TestOutcome,
) -> String {
	let prefix = outcome_prefix(params, outcome);
	test_heading_log(params, &prefix, test)
}

fn test_heading_log(
	_params: &LoggerParams,
	prefix: &str,
	test: &Test,
) -> String {
	// format!(
	// 	"{} {}:{}:{} - {}\n",
	// 	prefix, test.source_file, test.start_line, test.start_col, test.name
	// )
	format!("{} {}", prefix, test.short_file_and_name())
}


pub(super) fn log_final(
	requests: Populated<
		(Entity, &RequestMeta, &FinalOutcome, &Children),
		Added<FinalOutcome>,
	>,
	mut params: Extractor<LoggerParams>,
	tests: Query<(&Test, &TestOutcome)>,
) -> Result {
	for (entity, req, outcome, children) in requests {
		let params = params.get(entity)?;
		if params.quiet {
			continue;
		}
		let mut out = Vec::new();
		if outcome.num_fail() != 0 {
			for (test, case_outcome) in
				children.iter().filter_map(|child| tests.get(child).ok())
			{
				if let TestOutcome::Fail(fail) = case_outcome {
					out.push(String::new());
					out.push(failed_heading(&params, test, fail));
					out.push(String::new());
					out.push(failed_file_context(&params, test, fail)?);
					out.push(String::new());
					out.push(failed_stacktrace(&params, test, fail));
					out.push(String::new());
				}
			}
		}
		out.push(summary_message(&params, outcome));
		out.push(String::new());
		out.push(run_stats(&params, outcome, req));
		beet_core::cross_log!("\n{}", out.join("\n"));
	}

	Ok(())
}

#[allow(unused)]
fn summary_message(params: &LoggerParams, outcome: &FinalOutcome) -> String {
	if outcome.num_fail() == 0 && outcome.num_ran() != 0 {
		let msg = "All tests passed";
		if params.no_color {
			msg.to_string()
		} else {
			Color::Cyan.bold().underline().paint(msg).to_string()
		}
	} else if outcome.num_ran() == 0 {
		let msg = "No tests ran";
		if params.no_color {
			msg.to_string()
		} else {
			Color::Red.bold().underline().paint(msg).to_string()
		}
	} else {
		let msg = format!("Some tests failed");
		if params.no_color {
			msg
		} else {
			Color::Red.bold().underline().paint(msg).to_string()
		}
	}
}

fn test_stats(params: &LoggerParams, outcome: &FinalOutcome) -> String {
	let mut stats = Vec::new();
	if outcome.num_fail() > 0 {
		let fail_msg = format!("{} failed", outcome.num_fail());
		if params.no_color {
			stats.push(fail_msg);
		} else {
			stats.push(Color::Red.bold().paint(fail_msg).to_string());
		}
	}
	if outcome.num_skip() > 0 {
		let skip_msg = format!("{} skipped", outcome.num_skip());
		if params.no_color {
			stats.push(skip_msg);
		} else {
			stats.push(Color::Yellow.bold().paint(skip_msg).to_string());
		}
	}
	if outcome.num_pass() > 0 {
		let pass_msg = format!("{} passed", outcome.num_pass());
		if params.no_color {
			stats.push(pass_msg);
		} else {
			stats.push(Color::Green.bold().paint(pass_msg).to_string());
		}
	}
	stats.join(", ")
}

fn run_stats(
	params: &LoggerParams,
	outcome: &FinalOutcome,
	req: &RequestMeta,
) -> String {
	let duration = req.started().elapsed();
	let time = time_ext::pretty_print_duration(duration);
	let time = if params.no_color {
		time
	} else {
		Color::Blue.bold().paint(time).to_string()
	};
	let test_stats = test_stats(params, outcome);
	format!("{} in {}", test_stats, time)
}

fn failed_file_context(
	params: &LoggerParams,
	test: &Test,
	outcome: &TestFail,
) -> Result<String> {
	const LINE_CONTEXT_SIZE: usize = 2;
	const TAB_SPACES: usize = 2;


	let path = test.path().into_abs();
	let file = fs_ext::read_to_string(path)?;
	let lines = file.split('\n').collect::<Vec<_>>();
	let max_digits = lines.len().to_string().len();

	let start = outcome.start(test);

	let mut output = Vec::new();
	//line number is one-indexed
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
		let mut prefix = String::from(if is_err_line { ">" } else { " " });
		if !params.no_color {
			prefix = Color::Red.paint(prefix).to_string();
		}

		let buffer = {
			let line_digits = curr_line_no.to_string().len();
			let len = max_digits.saturating_sub(line_digits);
			" ".repeat(len)
		};
		let mut line_prefix =
			String::from(format!("{}{}|", curr_line_no, buffer));
		if !params.no_color {
			line_prefix = Style::new().dimmed().paint(line_prefix).to_string();
		}

		// replace tabs with spaces
		let line_with_spaces = lines[i].replace("\t", &" ".repeat(TAB_SPACES));

		// let prefix_len = 6;
		output.push(format!("{} {}{}", prefix, line_prefix, line_with_spaces));
		if is_err_line {
			let mut empty_line_prefix =
				format!("{}|", " ".repeat(2 + max_digits));
			if !params.no_color {
				empty_line_prefix =
					Style::new().dimmed().paint(empty_line_prefix).to_string();
			}
			let col_buffer = " ".repeat(start.col() as usize);
			let up_arrow = if params.no_color {
				"^".to_string()
			} else {
				Color::Red.paint("^").to_string()
			};
			output.push(format!(
				"{}{}{}",
				empty_line_prefix, col_buffer, up_arrow
			));
		}
	}

	output.join("\n").xok()
}
fn failed_heading(
	params: &LoggerParams,
	test: &Test,
	outcome: &TestFail,
) -> String {
	let mut title = test.short_file_and_name();
	let reason = fail_reason(params, outcome);

	if !params.no_color {
		title = Color::Red.paint(title).to_string();
	}
	format!("{}\n{}", title, reason)
}

fn fail_reason(params: &LoggerParams, outcome: &TestFail) -> String {
	match outcome {
		TestFail::Err { message } => {
			let mut prefix = "Returned error:".to_string();
			if !params.no_color {
				prefix = Style::new().bold().paint(prefix).to_string();
			}
			format!("{} {}", prefix, message)
		}
		TestFail::ExpectedPanic { message } => {
			if let Some(message) = message {
				let mut prefix = "Expected panic:".to_string();
				if !params.no_color {
					prefix = Style::new().bold().paint(prefix).to_string();
				}
				format!("{} {}", prefix, message)
			} else {
				let mut prefix = "Expected panic".to_string();
				if !params.no_color {
					prefix = Style::new().bold().paint(prefix).to_string();
				}
				prefix
			}
		}
		TestFail::Panic { payload, .. } => {
			if let Some(payload) = payload {
				let mut prefix = "Panicked:".to_string();
				if !params.no_color {
					prefix = Style::new().bold().paint(prefix).to_string();
				}
				format!("{} {}", prefix, payload)
			} else {
				let mut prefix = "Panicked - opaque payload".to_string();
				if !params.no_color {
					prefix = Style::new().bold().paint(prefix).to_string();
				}
				prefix
			}
		}
	}
}

fn failed_stacktrace(
	params: &LoggerParams,
	test: &Test,
	outcome: &TestFail,
) -> String {
	let mut prefix = String::from("at");
	let mut path = test.path().to_string();
	let start = outcome.start(test);
	let mut line_loc = format!(":{}:{}", start.line(), start.col());

	if !params.no_color {
		prefix = Style::new().dimmed().paint(prefix).to_string();
		path = Color::Cyan.paint(path).to_string();
		line_loc = Style::new().dimmed().paint(line_loc).to_string();
	}
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
