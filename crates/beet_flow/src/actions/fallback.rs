use crate::prelude::*;
use beet_core::prelude::*;

/// Aka `Selector`, runs all children in order until one succeeds.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Logic
/// - If a child succeeds it will succeed.
/// - If a child fails it will run the next child.
/// - If there are no more children to run it will fail.
/// ## Example
/// Runs the first child, then the second child if first fails.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = BeetFlowPlugin::world();
///	world.spawn((
/// 	Fallback,
/// 	children![
/// 		EndWith(Outcome::Fail),
/// 		EndWith(Outcome::Pass),
///    ]))
///		.trigger_action(GetOutcome)
/// 	.flush();
/// ```
#[action(on_start, on_next)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct Fallback;

fn on_start(
	ev: On<GetOutcome>,
	mut commands: Commands,
	query: Query<&Children>,
) -> Result {
	let children = query.get(ev.event_target())?;
	if let Some(first_child) = children.iter().next() {
		commands.entity(first_child).trigger_action(GetOutcome);
	} else {
		commands
			.entity(ev.event_target())
			.trigger_action(Outcome::Fail);
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
	// if any success, propagate the success
	if ev.is_pass() {
		ChildEnd::propagate(commands, &ev);
		return Ok(());
	}
	let children = query.get(target)?;
	let index = children
		.iter()
		.position(|x| x == child)
		.ok_or_else(|| expect_action::to_have_child(&ev, child))?;
	if index == children.len() - 1 {
		// all done, propagate the failure
		ChildEnd::propagate(commands, &ev);
	} else {
		// run next
		commands
			.entity(children[index + 1])
			.trigger_action(GetOutcome);
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
			.spawn((Name::new("root"), Fallback, children![
				(Name::new("child1"), EndWith(Outcome::Fail)),
				(Name::new("child2"), EndWith(Outcome::Pass)),
			]))
			.trigger_action(GetOutcome)
			.flush();

		on_run.get().xpect_eq(vec![
			"root".to_string(),
			"child1".to_string(),
			"child2".to_string(),
		]);
		on_result.get().xpect_eq(vec![
			("child1".to_string(), Outcome::Fail),
			("child2".to_string(), Outcome::Pass),
			("root".to_string(), Outcome::Pass),
		]);
	}
}
