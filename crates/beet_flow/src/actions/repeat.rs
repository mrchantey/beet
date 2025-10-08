use crate::prelude::*;
use beet_core::prelude::*;

/// Reattaches the [`RunOnSpawn`] component whenever [`End`] is called.
/// Using [`RunOnSpawn`] means this does **not** directly trigger observers, which avoids infinite loops.
///
/// Note that [`Repeat`] requires [`PreventPropagateEnd`] so results must be bubbled up manually
/// if the [`Self::if_result_matches`] option is unused.
///
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Example
/// Repeat the action twice, then bubble up the failure
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = BeetFlowPlugin::world();
/// world
/// .spawn((Repeat::if_success(), SucceedTimes::new(2)))
/// .trigger_target(GetOutcome);
/// ```
#[action(repeat)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct Repeat {
	/// Optional predicate to only repeat if the result matches.
	pub if_result_matches: Option<Outcome>,
}

impl Repeat {
	/// Repeats the action if the result is [`Outcome::Pass`].
	pub fn if_success() -> Self {
		Self {
			if_result_matches: Some(Outcome::Pass),
		}
	}
	/// Repeats the action if the result is [`Outcome::Fail`].
	pub fn if_failure() -> Self {
		Self {
			if_result_matches: Some(Outcome::Fail),
		}
	}
}

impl Default for Repeat {
	fn default() -> Self {
		Self {
			if_result_matches: None,
		}
	}
}

fn repeat(
	ev: On<Outcome>,
	query: Query<&Repeat>,
	mut commands: Commands,
) -> Result {
	let repeat = query.get(ev.event_target())?;
	if let Some(check) = &repeat.if_result_matches {
		if *ev != *check {
			// repeat is completed, propagate the result to the parent if it exists
			ChildEnd::trigger(commands, &ev);
			return Ok(());
		}
	}
	// otherwise run again on the next tick
	commands
		.entity(ev.event_target())
		.insert(TriggerDeferred::new(GetOutcome));
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn repeat_always() {
		let mut world = BeetFlowPlugin::world();
		let on_result = collect_on_result(&mut world);

		world
			.spawn((Repeat::default(), SucceedTimes::new(2)))
			.trigger_target(GetOutcome)
			.flush();

		on_result.get().len().xpect_eq(1);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(2);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(3);
		world.run_schedule(Update);
		// even though child failed, it keeps repeating
		on_result.get().len().xpect_eq(4);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(5);
	}

	#[test]
	fn repeat_if() {
		let mut world = BeetFlowPlugin::world();
		let on_result = collect_on_result(&mut world);

		world
			.spawn((Repeat::if_success(), SucceedTimes::new(2)))
			.trigger_target(GetOutcome)
			.flush();

		on_result.get().len().xpect_eq(1);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(2);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(3);
		world.run_schedule(Update);
		// it stopped repeating
		on_result.get().len().xpect_eq(3);
		world.run_schedule(Update);
		on_result.get().len().xpect_eq(3);
	}

	#[test]
	fn repeat_child() {
		let mut world = BeetFlowPlugin::world();
		let on_result = collect_on_result(&mut world);

		world
			.spawn((Sequence, children![(
				Repeat::if_success(),
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
		// last one, it stopped repeating
		on_result.get().len().xpect_eq(7);
	}
}
