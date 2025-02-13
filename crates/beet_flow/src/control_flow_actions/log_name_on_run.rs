use crate::prelude::*;
use bevy::prelude::*;

/// Logs the [`Name`] of the entity when it runs.
/// ## Tags
/// - [InputOutput](ActionTag::InputOutput)
/// ## Example
/// ```
/// # use beet_flow::prelude::*;
/// # let mut world = world();
/// world
///		.spawn((Name::new("root"), LogNameOnRun))
///		.trigger(OnRun::local());
/// ```
#[action(log_name_on_run)]
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(Name)]
pub struct LogNameOnRun;


/// Logs the [`Name`] of the entity when it runs.
fn log_name_on_run(trigger: Trigger<OnRun>, query: Query<&Name>) {
	if let Ok(name) = query.get(trigger.action) {
		log::info!("Running: {name}");
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::prelude::*;

	/// run with `--nocapture` to check output
	#[test]
	fn action() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		app.world_mut()
			.spawn((Name::new("root"), LogNameOnRun))
			.flush_trigger(OnRun::local());

		Ok(())
	}
}
