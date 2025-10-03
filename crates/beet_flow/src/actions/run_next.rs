use crate::prelude::*;
use beet_core::prelude::*;

/// Chain runs together, even if they are not in the same hierarchy,
/// this is useful for a State Machine pattern, but be aware that
/// in terms of control flow this is essentially a [`goto`](https://xkcd.com/292/) statement.
///
/// The `origin` will be preserved in calling the next `Run`.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Example
/// Triggering the second action will run the first `action`.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = BeetFlowPlugin::world();
/// let action = world
/// 	.spawn(EndOnRun(SUCCESS))
/// 	.id();
/// world
/// 	.spawn((
/// 		EndOnRun(SUCCESS),
/// 		RunNext::new(action)
/// 	))
/// 	.trigger_payload(RUN);
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
	/// Create a new RunNext action that only runs if the result is [`Outcome::Success`].
	pub fn if_success(action: Entity) -> Self {
		Self {
			action,
			if_result_matches: Some(SUCCESS),
		}
	}
	/// Create a new RunNext action that only runs if the result is [`Outcome::Failure`].
	pub fn if_failure(action: Entity) -> Self {
		Self {
			action,
			if_result_matches: Some(FAILURE),
		}
	}
}

fn run_next(
	ev: On<End>,
	mut commands: Commands,
	query: Query<&RunNext>,
) -> Result {
	let run_next = query
		.get(ev.event_target())
		.expect(&expect_action::to_have_action(&ev));
	if let Some(check) = &run_next.if_result_matches {
		if **ev != *check {
			return Ok(());
		}
	}
	commands.entity(run_next.action).trigger_payload(RUN);
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
		let on_run = collect_on_run(&mut world);

		let action1 = world
			.spawn((Name::new("action1"), EndOnRun(SUCCESS)))
			.id();
		world
			.spawn((
				Name::new("action2"),
				RunNext::new(action1),
				EndOnRun(SUCCESS),
			))
			.trigger_payload(RUN)
			.flush();

		on_run
			.get()
			.xpect_eq(vec!["action2".to_string(), "action1".to_string()]);
		on_result.get().xpect_eq(vec![
			("action2".to_string(), SUCCESS),
			("action1".to_string(), SUCCESS),
		]);
	}
}
