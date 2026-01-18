use crate::prelude::*;
use crate::testing::runner::*;


/// Exits when all tests have finished
pub fn exit_on_suite_outcome(
	mut commands: Commands,
	finished: Populated<(Entity, &SuiteOutcome), Added<SuiteOutcome>>,
	mut exit_params: ParamQuery<RunnerParams>,
) -> Result {
	if let Some((entity, outcome)) = finished.iter().next() {
		let params = exit_params.get(entity)?;
		let exit = if params.watch || outcome.num_fail() == 0 {
			AppExit::Success
		} else {
			AppExit::error()
		};
		commands.write_message(exit);
	}
	Ok(())
}
