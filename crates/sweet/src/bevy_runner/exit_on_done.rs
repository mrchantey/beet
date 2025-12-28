use crate::bevy_runner::LoggerParams;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::RequestMeta;





/// Exits when all tests have finished
pub fn exit_on_done(
	mut commands: Commands,
	finished: Populated<(&RequestMeta, &FinalOutcome), Added<FinalOutcome>>,
) -> Result {
	if let Some((req, outcome)) = finished.iter().next() {
		let _params = req.params().parse::<LoggerParams>()?;
		
		
		let num_fail = outcome.num_fail();
		if num_fail == 0 {
			beet_core::cross_log!("All Tests Passed");
		} else {
			beet_core::cross_log!("{} Tests Failed", num_fail);
		};
		let exit = if num_fail == 0 {
			AppExit::Success
		} else {
			AppExit::error()
		};
		commands.write_message(exit);
	}
	Ok(())
}
