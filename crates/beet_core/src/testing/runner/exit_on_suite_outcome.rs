use crate::prelude::*;
use crate::testing::runner::*;


/// Exits when all tests have finished
pub fn exit_on_suite_outcome(
	mut commands: Commands,
	finished: Populated<
		(Entity, &TestRunnerConfig, &SuiteOutcome),
		Added<SuiteOutcome>,
	>,
) -> Result {
	if let Some((_entity, config, outcome)) = finished.iter().next() {
		let exit = if config.watch || outcome.num_fail() == 0 {
			AppExit::Success
		} else {
			AppExit::error()
		};
		commands.write_message(exit);
	}
	Ok(())
}
