use crate::prelude::*;
use beet_core::prelude::*;






pub fn collect_outcomes(
	mut commands: Commands,
	// only run when something added
	_added: Populated<(), Added<TestOutcome>>,
	tests: Query<&TestOutcome, Added<TestOutcome>>,
	outcomes: Query<(&Test, &TestOutcome), Added<TestOutcome>>,
) {
	if tests.count() == outcomes.count() {
		let mut failed = 0;
		for (test, outcome) in outcomes.iter().filter(|(_, res)| !res.is_pass())
		{
			// TODO proper output
			beet_core::cross_log!(
				" FAIL  {}#{} - {}\n{:?}",
				test.source_file, test.start_line, test.name, outcome
			);
			failed += 1;
		}

		let exit = if failed == 0 {
			beet_core::cross_log!("All Tests Passed");
			AppExit::Success
		} else {
			beet_core::cross_log!("{} Tests Failed", failed);
			AppExit::error()
		};
		commands.write_message(exit);
	}
}
