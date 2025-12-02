use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_rsx::prelude::*;


/// A cargo command
#[template]
pub fn BuildStep(
	/// The cargo command to run
	cmd: CargoBuildCmd,
) -> impl Bundle {
	OnSpawn::observe(|_ev: On<GetOutcome>, mut _commands: AsyncCommands| {
		todo!("async actions");
	})
}
