use crate::prelude::*;
use beet_core::prelude::*;

/// Chain runs together, even if they are not in the same hierarchy,
/// this is useful for a State Machine pattern, but be aware that
/// in terms of control flow this is essentially a [`goto`](https://xkcd.com/292/) statement.
///
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Example
/// Triggering the second action will run the first `action`.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = ControlFlowPlugin::world();
/// let action = world
/// 	.spawn(EndWith(Outcome::Pass))
/// 	.id();
/// world
/// 	.spawn((
/// 		EndWith(Outcome::Pass),
/// 		RunNext::new(action)
/// 	))
/// 	.trigger_target(GetOutcome);
/// ```
#[action(run_next)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct RunNext {
	/// The next action to run.
	pub action: Entity,
	/// if set, this will only run next if the result matches this,
	/// otherwise it will stop repeating and trigger End
	/// on its parent.
	pub if_result_matches: Option<Outcome>,
}

impl RunNext {
	/// Create a new RunNext action.
	pub fn new(action: Entity) -> Self {
		Self {
			action,
			if_result_matches: None,
		}
	}
	/// Create a new RunNext action that only runs if the result is [`Outcome::Pass`].
	pub fn if_success(action: Entity) -> Self {
		Self {
			action,
			if_result_matches: Some(Outcome::Pass),
		}
	}
	/// Create a new RunNext action that only runs if the result is [`Outcome::Fail`].
	pub fn if_failure(action: Entity) -> Self {
		Self {
			action,
			if_result_matches: Some(Outcome::Fail),
		}
	}
}

fn run_next(
	ev: On<Outcome>,
	mut commands: Commands,
	query: Query<&RunNext>,
) -> Result {
	let run_next = query
		.get(ev.target())
		.expect(&expect_action::to_have_action(&ev));
	if let Some(check) = &run_next.if_result_matches {
		if *ev != *check {
			return Ok(());
		}
	}
	commands.entity(run_next.action).trigger_target(GetOutcome);
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
		let on_run = collect_on_run(&mut world);

		let action1 = world
			.spawn((Name::new("action1"), EndWith(Outcome::Pass)))
			.id();
		world
			.spawn((
				Name::new("action2"),
				RunNext::new(action1),
				EndWith(Outcome::Pass),
			))
			.trigger_target(GetOutcome)
			.flush();

		on_run
			.get()
			.xpect_eq(vec!["action2".to_string(), "action1".to_string()]);
		on_result.get().xpect_eq(vec![
			("action2".to_string(), Outcome::Pass),
			("action1".to_string(), Outcome::Pass),
		]);
	}
}
