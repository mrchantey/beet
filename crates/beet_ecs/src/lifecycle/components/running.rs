use bevy::prelude::*;
use std::fmt::Debug;

/// Indicate this node is currently running.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Copy, Clone, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default)]
pub struct Running;




/// Indicate the result of an action.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Clone, Copy, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default)]
pub enum RunResult {
	#[default]
	/// The Action was successful.
	Success,
	/// The Action was unsuccessful.
	Failure,
}


/// Syncs [`Running`] and [`RunResult`] components, by default added to [`PostNodeUpdateSet`].
pub fn sync_running(
	mut commands: Commands,
	// occurs immediately after `RunResult` is added
	first_pass: Query<Entity, (Added<RunResult>, With<Running>)>,
	// occurs one frame later
	second_pass: Query<Entity, (With<RunResult>, Without<Running>)>,
) {
	for entity in first_pass.iter() {
		commands.entity(entity).remove::<Running>();
	}
	for entity in second_pass.iter() {
		commands.entity(entity).remove::<RunResult>();
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	pub fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(LifecyclePlugin);

		let entity = app.world_mut().spawn((Running, RunResult::Success)).id();

		expect(&app).to_have_component::<Running>(entity)?;
		// add `RunResult`, remove `Running`
		app.update();
		expect(&app).not().to_have_component::<Running>(entity)?;
		expect(&app).to_have_component::<RunResult>(entity)?;
		// remove `Running`
		app.update();
		// remove `RunResult`
		expect(&app).not().to_have_component::<Running>(entity)?;
		expect(&app).not().to_have_component::<RunResult>(entity)?;

		Ok(())
	}
}
