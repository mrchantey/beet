use crate::prelude::*;
use beet_core::prelude::*;

/// An action that runs all of its children in order until one fails.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Logic
/// - If a child fails it will fail.
/// - If a child succeeds it will run the next child.
/// - If there are no more children to run it will succeed.
/// ## Example
/// Runs the first child, then the second child.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = BeetFlowPlugin::world();
///	world.spawn((
/// 	Sequence,
/// 	children![
/// 		EndWith(Outcome::Pass),
/// 		EndWith(Outcome::Pass),
///    ]))
///		.trigger_target(GetOutcome)
/// 	.flush();
/// ```
#[action(on_start, on_next)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct Sequence;

fn on_start(
	ev: On<GetOutcome>,
	mut commands: Commands,
	query: Query<&Children>,
) -> Result {
	let children = query.get(ev.event_target())?;
	if let Some(first_child) = children.iter().next() {
		commands.entity(first_child).trigger_target(GetOutcome);
	} else {
		commands
			.entity(ev.event_target())
			.trigger_target(Outcome::Pass);
	}
	Ok(())
}

fn on_next(
	ev: On<ChildEnd<Outcome>>,
	mut commands: Commands,
	query: Query<&Children>,
) -> Result {
	let target = ev.event_target();
	let child = ev.child();
	// if any error, just propagate the error
	if ev.is_fail() {
		ChildEnd::propagate(commands, &ev);
		return Ok(());
	}
	let children = query.get(target)?;
	let index = children
		.iter()
		.position(|x| x == child)
		.ok_or_else(|| expect_action::to_have_child(&ev, child))?;
	if index == children.len() - 1 {
		// all done, propagate the success
		ChildEnd::propagate(commands, &ev);
	} else {
		// run next
		commands
			.entity(children[index + 1])
			.trigger_target(GetOutcome);
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
		let on_run = collect_on_run(&mut world);

		world
			.spawn((Name::new("root"), Sequence, children![
				(Name::new("child1"), EndWith(Outcome::Pass)),
				(Name::new("child2"), EndWith(Outcome::Fail)),
			]))
			.trigger_target(GetOutcome)
			.flush();

		on_run.get().xpect_eq(vec![
			"root".to_string(),
			"child1".to_string(),
			"child2".to_string(),
		]);
		on_result.get().xpect_eq(vec![
			("child1".to_string(), Outcome::Pass),
			("child2".to_string(), Outcome::Fail),
			("root".to_string(), Outcome::Fail),
		]);
	}
}
