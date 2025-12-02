use beet_core::prelude::*;

/// A cargo command 
#[derive(Debug, Clone, Component)]
pub struct BuildStep {
	/// The cargo command to run
	pub cmd: CargoBuildCmd,
}



/// extend
pub async fn try_load_beet_lock(world: AsyncWorld) {}
