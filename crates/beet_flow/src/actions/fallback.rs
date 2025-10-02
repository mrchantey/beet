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
/// # use beet_flow::prelude::*;
/// # let mut world = BeetFlowPlugin::world();
///	world.spawn((
/// 	Fallback,
/// 	children![
/// 		EndOnRun::failure(),
/// 		EndOnRun::success(),
///    ]))
///		.trigger_entity(RUN)
/// 	.flush();
/// ```
#[action(on_start, on_next)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct Fallback;

fn on_start(
	ev: On<Run>,
	mut commands: Commands,
	query: Query<&Children>,
) -> Result {
	let children = query.get(ev.event_target())?;
	if let Some(first_child) = children.iter().next() {
		commands.entity(first_child).trigger_entity(RUN);
	} else {
		commands
			.entity(ev.event_target())
			.trigger_entity(IntoEnd::failure());
	}
	Ok(())
}

fn on_next(
	ev: On<ChildEnd>,
	mut commands: Commands,
	query: Query<&Children>,
) -> Result {
	let target = ev.event_target();
	let child = ev.child();
	// if any success, propagate the success
	if ev.is_ok() {
		commands.trigger(ev.event().clone().into_end());
		return Ok(());
	}
	let children = query.get(target)?;
	let index = children
		.iter()
		.position(|x| x == child)
		.ok_or_else(|| expect_action::to_have_child(&ev, child))?;
	if index == children.len() - 1 {
		// all done, propagate the failure
		commands.trigger(ev.event().clone().into_end());
	} else {
		// run next
		commands.entity(children[index + 1]).trigger_entity(RUN);
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
				(Name::new("child1"), EndOnRun::failure()),
				(Name::new("child2"), EndOnRun::success()),
			]))
			.trigger_entity(RUN)
			.flush();

		on_run.get().xpect_eq(vec![
			"root".to_string(),
			"child1".to_string(),
			"child2".to_string(),
		]);
		on_result.get().xpect_eq(vec![
			("child1".to_string(), Err(())),
			("child2".to_string(), Ok(())),
			("root".to_string(), Ok(())),
		]);
	}
}
