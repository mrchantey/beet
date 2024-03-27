use bevy::prelude::*;
use std::fmt::Debug;
use strum_macros::Display;
use strum_macros::EnumIter;


/// Indicate this node is currently running.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Copy, Clone, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
pub struct Running;




/// Indicate the result of an action.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(
	Default,
	Debug,
	Clone,
	Copy,
	Component,
	PartialEq,
	EnumIter,
	Display,
	Reflect,
)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
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
		app.add_plugins(BeetSystemsPlugin::<EcsNode, _>::default());

		let target = app.world.spawn_empty().id();

		let root = InsertOnRun(RunResult::Success)
			.into_beet_node()
			.spawn(&mut app.world, target)
			.value;

		expect(&app).to_have_component::<Running>(root)?;
		// add `RunResult`, remove `Running`
		app.update();
		expect(&app).not().to_have_component::<Running>(root)?;
		expect(&app).to_have_component::<RunResult>(root)?;
		// remove `Running`
		app.update();
		// remove `RunResult`
		expect(&app).not().to_have_component::<Running>(root)?;
		expect(&app).not().to_have_component::<RunResult>(root)?;

		Ok(())
	}
}
