use crate::prelude::*;
use bevy::prelude::*;



/// When [`OnRunResult`] is triggered, propagate to parent with [`OnChildResult`].
/// We can't use bevy event propagation because that does not track the 'bubbler'.
pub fn bubble_run_result(
	trigger: Trigger<OnRunResult>,
	mut commands: Commands,
	parents: Query<&Parent>,
) {
	if let Some(parent) = parents.get(trigger.entity()).ok() {
		commands.entity(parent.get()).trigger(OnChildResult::new(
			trigger.entity(),
			trigger.event().result(),
		));
	}
}


/// Add this to flow actions to pass the run result to the parent.
pub fn passthrough_run_result(
	trigger: Trigger<OnChildResult>,
	mut commands: Commands,
) {
	commands
		.entity(trigger.entity())
		.trigger(OnRunResult::new(*trigger.event().value()));
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevyhub::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<EndOnRun>::default());
		let world = app.world_mut();
		world.add_observer(bubble_run_result);
		let on_run = observe_triggers::<OnRun>(world);
		let on_run_result = observe_triggers::<OnRunResult>(world);

		world
			.spawn((Name::new("root"), EndOnRun::success()))
			.observe(passthrough_run_result)
			.with_children(|parent| {
				parent
					.spawn((Name::new("child1"), EndOnRun::success()))
					// child starts running which triggers parent
					.flush_trigger(OnRun);
			});

		expect(&on_run).to_have_been_called_times(1)?;
		expect(&on_run_result).to_have_been_called_times(2)?;

		Ok(())
	}
}
