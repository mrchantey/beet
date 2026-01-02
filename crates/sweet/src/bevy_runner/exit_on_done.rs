use crate::prelude::*;
use beet_core::prelude::*;





/// Exits when all tests have finished
pub fn exit_on_done(
	mut commands: Commands,
	finished: Populated<&SuiteOutcome, Added<SuiteOutcome>>,
) -> Result {
	if let Some(outcome) = finished.iter().next() {
		let exit = if outcome.num_fail() == 0 {
			AppExit::Success
		} else {
			AppExit::error()
		};
		commands.write_message(exit);
	}
	Ok(())
}
