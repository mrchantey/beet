//! Re-triggering actions based on their results.
use crate::prelude::*;
use beet_core::prelude::*;

/// Reattaches the [`TriggerDeferred::get_outcome()`] component whenever [`End`] is called.
/// Using [`TriggerDeferred::get_outcome()`] means this does **not** directly trigger observers, which avoids infinite loops.
///
/// Note that [`Retrigger`] requires [`PreventPropagateEnd`] so results must be bubbled up manually
/// if the [`Self::if_result_matches`] option is unused.
///
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Example
/// Retrigger the action twice, then bubble up the failure
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = ControlFlowPlugin::world();
/// world
/// .spawn((Retrigger::if_success(), SucceedTimes::new(2)))
/// .trigger_target(GetOutcome);
/// ```
#[action(retrigger)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct Retrigger {
	/// Optional predicate to only retrigger if the result matches.
	pub if_result_matches: Option<Outcome>,
}

impl Retrigger {
	/// Retriggers the action if the result is [`Outcome::Pass`].
	pub fn if_success() -> Self {
		Self {
			if_result_matches: Some(Outcome::Pass),
		}
	}
	/// Retriggers the action if the result is [`Outcome::Fail`].
	pub fn if_failure() -> Self {
		Self {
			if_result_matches: Some(Outcome::Fail),
		}
	}
}

impl Default for Retrigger {
	fn default() -> Self {
		Self {
			if_result_matches: None,
		}
	}
}

fn retrigger(
	ev: On<Outcome>,
	query: Query<&Retrigger>,
	mut commands: Commands,
) -> Result {
	let target = ev.target();
	let retrigger = query.get(target)?;
	if let Some(check) = &retrigger.if_result_matches {
		if *ev != *check {
			// retrigger is completed, propagate the result to the parent if it exists
			ChildEnd::trigger(commands, target, ev.event().clone());
			return Ok(());
		}
	}
	// otherwise run again on the next tick
	commands
		.entity(target)
		.insert(TriggerDeferred::get_outcome());
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn retrigger_always() {
		let mut world = ControlFlowPlugin::world();
		let on_result = collect_on_result(&mut world);

		world
			.spawn((Retrigger::default(), SucceedTimes::new(2)))
			.trigger_target(GetOutcome)
			.flush();

		on_result.get().len().xpect_eq(1);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(2);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(3);
		world.run_schedule(Update);
		// even though child failed, it keeps retriggering
		on_result.get().len().xpect_eq(4);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(5);
	}

	#[test]
	fn retrigger_if() {
		let mut world = ControlFlowPlugin::world();
		let on_result = collect_on_result(&mut world);

		world
			.spawn((Retrigger::if_success(), SucceedTimes::new(2)))
			.trigger_target(GetOutcome)
			.flush();

		on_result.get().len().xpect_eq(1);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(2);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(3);
		world.run_schedule(Update);
		// it stopped retriggering
		on_result.get().len().xpect_eq(3);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(3);
	}

	#[test]
	fn retrigger_child() {
		let mut world = ControlFlowPlugin::world();
		let on_result = collect_on_result(&mut world);

		world
			.spawn((Sequence, children![(
				Retrigger::if_success(),
				SucceedTimes::new(2)
			)]))
			.trigger_target(GetOutcome)
			.flush();

		on_result.get().len().xpect_eq(2);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(4);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(7);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(7);
		world.run_schedule(Update);
		// last one, it stopped retriggering
		on_result.get().len().xpect_eq(7);
	}
}
