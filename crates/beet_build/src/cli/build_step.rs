use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_rsx::prelude::*;


/// A cargo command
#[template]
pub fn BuildStep(
	/// The cargo command to run
	cmd: CargoBuildCmd,
) -> impl Bundle {
	OnSpawn::observe(|ev: On<GetOutcome>, mut commands: AsyncCommands| {
		let action = ev.action();
		commands.run(async |world| {
			world.trigger();
			
		});
		// ev.trigg
	})
}
