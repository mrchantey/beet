use crate::prelude::*;
use beet_core::prelude::*;

/// An action that runs all of its children in order no matter what they return.
/// When the last child finishes it will return [`Outcome::Pass`]
/// ## Example
/// Runs the first child, then the second child, then the third,
/// and finally triggering [`Outcome::Pass`] on the root.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = ControlFlowPlugin::world();
///	world.spawn((
/// 	InfallibleSequence,
/// 	children![
/// 		EndWith(Outcome::Fail),
/// 		EndWith(Outcome::Pass),
/// 		EndWith(Outcome::Fail),
///    ]))
///		.trigger_target(GetOutcome)
/// 	.flush();
/// ```
#[action(on_start, on_next)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct InfallibleSequence;

fn on_start(mut ev: On<GetOutcome>, query: Query<&Children>) -> Result {
	let children = query.get(ev.action())?;
	if let Some(first_child) = children.iter().next() {
		ev.trigger_action_with_cx(first_child, GetOutcome);
	} else {
		ev.trigger_with_cx(Outcome::Pass);
	}
	Ok(())
}

fn on_next(mut ev: On<ChildEnd<Outcome>>, query: Query<&Children>) -> Result {
	let target = ev.action();
	let child = ev.child();
	let children = query.get(target)?;
	let index = children
		.iter()
		.position(|x| x == child)
		.ok_or_else(|| expect_action::to_have_child(&ev, child))?;
	if index == children.len() - 1 {
		// all done, return pass
		ev.trigger_with_cx(Outcome::Pass);
	} else {
		// run next
		ev.trigger_action_with_cx(children[index + 1], GetOutcome);
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
