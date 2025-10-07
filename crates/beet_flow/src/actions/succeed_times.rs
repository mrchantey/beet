use crate::prelude::*;
use beet_core::prelude::*;

/// Succeed a certain number of times before failing.
/// ## Tags
/// - [`ControlFlow`](ActionTag::ControlFlow)
/// - [`LongRunning`](ActionTag::LongRunning)
///
/// For example usage see [`Repeat`].
#[action(succeed_times)]
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct SucceedTimes {
	/// The number of times the action has executed.
	pub times: usize,
	/// The number of times the action should succeed before failing.
	pub max_times: usize,
}

impl SucceedTimes {
	/// Specify the number of times the action should succeed before failing.
	pub fn new(max_times: usize) -> Self {
		Self {
			times: 0,
			max_times,
		}
	}
	/// Reset the number of times the action has executed.
	pub fn reset(&mut self) { self.times = 0; }
}

fn succeed_times(
	ev: On<Run>,
	mut commands: Commands,
	mut query: Query<&mut SucceedTimes>,
) -> Result {
	let mut action = query.get_mut(ev.event_target())?;

	if action.times < action.max_times {
		action.times += 1;
		commands.entity(ev.event_target()).trigger_payload(Outcome::Pass);
	} else {
		commands.entity(ev.event_target()).trigger_payload(Outcome::Fail);
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = BeetFlowPlugin::world();
		let on_result = collect_on_result(&mut world);

		let entity = world.spawn(SucceedTimes::new(2)).id();

		world.entity_mut(entity).trigger_payload(GetOutcome).flush();

		on_result
			.get()
			.xpect_eq(vec![("Unknown".to_string(), Outcome::Pass)]);
		world.entity_mut(entity).trigger_payload(GetOutcome).flush();
		on_result.get().xpect_eq(vec![
			("Unknown".to_string(), Outcome::Pass),
			("Unknown".to_string(), Outcome::Pass),
		]);
		world.entity_mut(entity).trigger_payload(GetOutcome).flush();
		on_result.get().xpect_eq(vec![
			("Unknown".to_string(), Outcome::Pass),
			("Unknown".to_string(), Outcome::Pass),
			("Unknown".to_string(), Outcome::Fail),
		]);
		world.entity_mut(entity).trigger_payload(GetOutcome).flush();
		on_result.get().xpect_eq(vec![
			("Unknown".to_string(), Outcome::Pass),
			("Unknown".to_string(), Outcome::Pass),
			("Unknown".to_string(), Outcome::Fail),
			("Unknown".to_string(), Outcome::Fail),
		]);
	}
}
