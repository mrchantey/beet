//! Debugging utility for actions that succeed a limited number of times.
use crate::prelude::*;
use beet_core::prelude::*;

/// A debugging utility that will succeed a given number of times, then fail.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Example
/// Succeed twice, then fail.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = ControlFlowPlugin::world();
/// world
/// 	.spawn(SucceedTimes::new(2))
/// 	.trigger_target(GetOutcome);
/// ```
#[action(succeed_times)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct SucceedTimes {
	/// The number of times to succeed
	pub max_times: u32,
	/// The current number of times this action has been run
	pub times: u32,
}
impl SucceedTimes {
	/// Create a new [`SucceedTimes`] action with the given number of times.
	pub fn new(max_times: u32) -> Self {
		Self {
			max_times,
			times: 0,
		}
	}
}

fn succeed_times(
	ev: On<GetOutcome>,
	mut commands: Commands,
	mut query: Query<&mut SucceedTimes>,
) -> Result {
	let target = ev.target();
	let mut action = query.get_mut(target)?;
	if action.times < action.max_times {
		action.times += 1;
		commands.entity(target).trigger_target(Outcome::Pass);
	} else {
		commands.entity(target).trigger_target(Outcome::Fail);
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		let mut world = ControlFlowPlugin::world();

		let on_result = collect_on_result(&mut world);

		let entity =
			world.spawn((Name::new("root"), SucceedTimes::new(2))).id();

		world.entity_mut(entity).trigger_target(GetOutcome).flush();

		on_result
			.get()
			.xpect_eq(vec![("root".to_string(), Outcome::Pass)]);
		world.entity_mut(entity).trigger_target(GetOutcome).flush();
		world.entity_mut(entity).trigger_target(GetOutcome).flush();
		world.entity_mut(entity).trigger_target(GetOutcome).flush();
		world.entity_mut(entity).trigger_target(GetOutcome).flush();
		world
			.query::<&SucceedTimes>()
			.single(&world)
			.unwrap()
			.times
			.xpect_eq(2);
	}
}
