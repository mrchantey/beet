use crate::prelude::*;
use beet_core::prelude::*;

/// Logs the [`Name`] of the entity when it runs.
/// ## Tags
/// - [InputOutput](ActionTag::InputOutput)
/// ## Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = BeetFlowPlugin::world();
/// world
///		.spawn((Name::new("root"), LogNameOnRun))
///		.trigger_action(GetOutcome);
/// ```
#[action(log_name_on_run)]
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(Name)]
pub struct LogNameOnRun;

/// Logs the [`Name`] of the entity when it runs.
fn log_name_on_run(ev: On<GetOutcome>, query: Query<&Name>) -> Result {
	if let Ok(name) = query.get(ev.event_target()) {
		log::info!("Running: {name}");
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// run with `--nocapture` to check output
	#[test]
	fn action() {
		let mut world = BeetFlowPlugin::world();
		world
			.spawn((Name::new("root"), LogNameOnRun))
			.trigger_action(GetOutcome)
			.flush();
	}
}
