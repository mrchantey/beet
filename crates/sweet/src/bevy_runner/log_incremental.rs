use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use yansi::Paint;


#[derive(Reflect)]
pub(super) struct LoggerParams {
	/// Do not log test outcomes as they complete
	no_incremental: bool,
	no_color: bool,
}

/// Collects test outcomes once all tests have finished running
pub fn log_incremental(
	requests: Populated<(Entity, &RequestMeta, &Children), Added<RequestMeta>>,
	just_finished: Populated<(&Test, &TestOutcome), Added<TestOutcome>>,
) -> Result {
	for (_entity, req, children) in requests {
		let params = req.params().parse::<LoggerParams>()?;
		if params.no_incremental {
			continue;
		}
		let just_finished = children
			.iter()
			.filter_map(|child| just_finished.get(child).ok())
			.collect::<Vec<_>>();

		for (test, outcome) in &just_finished {
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
	use nu_ansi_term::Color;
	match (outcome, params.no_color) {
		(TestOutcome::Pass, false) => {
			Color::Black.paint(" PASS ").bold().on_green().to_string()
		}
		(TestOutcome::Pass, true) => " PASS ".to_string(),
		(TestOutcome::Skip(_), false) => {
			Color::Yellow.paint(" SKIP ").bold().to_string()
		}
		(TestOutcome::Skip(_), true) => " SKIP ".to_string(),
		(TestOutcome::Fail(_), false) => {
			Color::Black.paint(" FAIL ").bold().on_red().to_string()
		}
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
	format!("{} {}", prefix, test.name)
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
			Request::from_cli_str("").unwrap(),
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
