use crate::prelude::*;
use beet_core::prelude::*;

/// Similar to [`Sequence`], but always succeeds regardless of child results.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Logic
/// - If a child fails it will run the next child.
/// - If a child succeeds it will run the next child.
/// - If there are no more children to run it will succeed.
/// ## Example
/// Runs all children regardless of their results.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = ControlFlowPlugin::world();
///	world.spawn((
/// 	InfallibleSequence,
/// 	children![
/// 		EndWith(Outcome::Fail),
/// 		EndWith(Outcome::Pass),
///    ]))
///		.trigger_target(GetOutcome)
/// 	.flush();
/// ```
#[action(on_start, on_next)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct InfallibleSequence;

fn on_start(
	ev: On<GetOutcome>,
	mut commands: Commands,
	query: Query<&Children>,
) -> Result {
	let target = ev.target();
	let children = query.get(target)?;
	if let Some(first_child) = children.iter().next() {
		commands.entity(first_child).trigger_target(GetOutcome);
	} else {
		commands.entity(target).trigger_target(Outcome::Pass);
	}
	Ok(())
}

fn on_next(
	ev: On<ChildEnd<Outcome>>,
	mut commands: Commands,
	query: Query<&Children>,
) -> Result {
	let target = ev.target();
	let child = ev.child();
	let children = query.get(target)?;
	let index = children
		.iter()
		.position(|x| x == child)
		.ok_or_else(|| expect_action::to_have_child(&ev, child))?;

	if index == children.len() - 1 {
		// all done, return pass
		commands.entity(target).trigger_target(Outcome::Pass);
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

	#[test]
	fn works() {
		let mut world = ControlFlowPlugin::world();

		let on_result = collect_on_result(&mut world);
		let on_run = collect_on_run(&mut world);

		world
			.spawn((Name::new("root"), InfallibleSequence, children![
				(Name::new("child1"), EndWith(Outcome::Fail)),
				(Name::new("child2"), EndWith(Outcome::Pass)),
				(Name::new("child3"), EndWith(Outcome::Fail)),
			]))
			.trigger_target(GetOutcome)
			.flush();

		on_run.get().xpect_eq(vec![
			"root".to_string(),
			"child1".to_string(),
			"child2".to_string(),
			"child3".to_string(),
		]);
		on_result.get().xpect_eq(vec![
			("child1".to_string(), Outcome::Fail),
			("child2".to_string(), Outcome::Pass),
			("child3".to_string(), Outcome::Fail),
			("root".to_string(), Outcome::Pass),
		]);
	}
}
