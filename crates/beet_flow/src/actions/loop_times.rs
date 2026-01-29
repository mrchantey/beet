use crate::prelude::*;
use beet_core::prelude::*;

/// Each n times this action runs, it will not trigger an Outcome,
/// instead triggering GetOutcome on the first child in its parent's [`Children`],
/// ie the first sibling.
/// ## Example
/// Succeed twice, then fail.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = ControlFlowPlugin::world();
/// // will fail on the fourth run
/// world
/// 	.spawn((Sequence, children![
///     SucceedTimes::new(3),
///     LoopTimes::new(100)
///   ]))
/// 	.trigger_target(GetOutcome);
/// ```
#[action(loop_times)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct LoopTimes {
	/// The number of times to succeed
	pub max_times: u32,
	/// The current number of times this action has been run
	pub times: u32,
}
impl LoopTimes {
	/// Create a new [`LoopTimes`] action with the given number of times.
	pub fn new(max_times: u32) -> Self {
		Self {
			max_times,
			times: 0,
		}
	}
}

fn loop_times(
	ev: On<GetOutcome>,
	mut commands: Commands,
	children: Query<&Children>,
	mut query: Query<(&mut LoopTimes, &ChildOf)>,
) -> Result {
	let target = ev.target();
	let (mut action, parent) = query.get_mut(target)?;
	if action.times < action.max_times {
		action.times += 1;
		let first = children.get(parent.get())?.first().ok_or_else(|| {
			bevyhow!("LoopTimes parent has no children to trigger")
		})?;
		commands.entity(*first).trigger_target(GetOutcome);
	} else {
		commands.entity(target).trigger_target(Outcome::Pass);
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	#[should_panic]
	fn no_parent() {
		let mut world = ControlFlowPlugin::world();
		world
			.spawn(LoopTimes::new(10))
			.trigger_target(GetOutcome)
			.flush();
	}
	#[test]
	fn works() {
		let mut world = ControlFlowPlugin::world();
		world
			.spawn((Sequence, ExitOnEnd, children![
				SucceedTimes::new(3),
				LoopTimes::new(10)
			]))
			.trigger_target(GetOutcome);
		world.run_local().xpect_eq(AppExit::error());
	}
	#[test]
	fn succeeds() {
		let mut world = ControlFlowPlugin::world();
		world
			.spawn((Sequence, ExitOnEnd, children![
				SucceedTimes::new(10),
				LoopTimes::new(3)
			]))
			.trigger_target(GetOutcome);
		world.run_local().xpect_eq(AppExit::Success);
	}
}
