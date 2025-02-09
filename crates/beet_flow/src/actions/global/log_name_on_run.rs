use crate::prelude::*;
use bevy::prelude::*;

/// Logs the [`Name`] of the entity when it runs.
#[derive(Default, Component, Action, Reflect)]
#[reflect(Default, Component)]
#[observers(log_name_on_run)]
pub struct LogNameOnRun;


/// Logs the [`Name`] of the entity when it runs.
pub fn log_name_on_run(trigger: Trigger<OnRun>, query: Query<&Name>) {
	if let Ok(name) = query.get(trigger.entity()) {
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
		World::new()
			.spawn((Name::new("root"), LogNameOnRun))
			.flush_trigger(OnRun);

		Ok(())
	}

	/// run with `--nocapture` to check output
	#[test]
	fn system() -> Result<()> {
		World::new()
			.with_observer(log_name_on_run)
			.spawn(Name::new("root"))
			.flush_trigger(OnRun);

		Ok(())
	}
}
