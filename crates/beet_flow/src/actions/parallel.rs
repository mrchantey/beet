use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::collections::HashSet;

/// An action that runs all of its children in parallel.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Logic
/// - If a child fails it will fail immediately.
/// - If all children succeed it will succeed.
/// ## Example
/// Run two children in parallel
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = BeetFlowPlugin::world();
/// world.spawn((
/// 	Parallel::default(),
/// 	children![
/// 		EndOnRun(SUCCESS),
/// 		EndOnRun(SUCCESS),
/// 	]))
/// 	.trigger_payload(RUN)
/// 	.flush();
/// ```
#[action(on_start, on_next)]
#[derive(Default, Component, Deref, DerefMut, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct Parallel(pub HashSet<Entity>);

fn on_start(
	ev: On<Run>,
	mut commands: Commands,
	mut query: Query<(&mut Parallel, &Children)>,
) -> Result {
	let (mut action, children) = query.get_mut(ev.event_target())?;
	action.clear();

	if children.is_empty() {
		commands.entity(ev.event_target()).trigger_payload(SUCCESS);
		return Ok(());
	}

	for child in children.iter() {
		commands.entity(child).trigger_payload(RUN);
	}
	Ok(())
}

fn on_next(
	ev: On<ChildEnd>,
	mut commands: Commands,
	mut query: Query<(&mut Parallel, &Children)>,
) -> Result {
	let target = ev.event_target();
	let child = ev.child();

	// if any error, just propagate the error
	if ev.is_failure() {
		commands.trigger(ev.event().clone().into_end());
		return Ok(());
	}

	let (mut action, children) = query.get_mut(target)?;
	action.insert(child);

	// if all children have completed successfully, succeed
	if action.len() == children.len() {
		commands.trigger(ev.event().clone().into_end());
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn fails() {
		let mut world = BeetFlowPlugin::world();

		let on_result = collect_on_result(&mut world);
		let on_run = collect_on_run(&mut world);

		world
			.spawn((Name::new("root"), Parallel::default(), children![
				(Name::new("child1"), EndOnRun(SUCCESS)),
				(Name::new("child2"), EndOnRun(FAILURE)),
			]))
			.trigger_payload(RUN)
			.flush();

		on_run.get().xpect_eq(vec![
			"root".to_string(),
			"child1".to_string(),
			"child2".to_string(),
		]);
		on_result.get().xpect_eq(vec![
			("child1".to_string(), SUCCESS),
			("child2".to_string(), FAILURE),
			("root".to_string(), FAILURE),
		]);
	}

	#[test]
	fn succeeds() {
		let mut world = BeetFlowPlugin::world();

		let on_result = collect_on_result(&mut world);
		let on_run = collect_on_run(&mut world);

		world
			.spawn((Name::new("root"), Parallel::default(), children![
				(Name::new("child1"), EndOnRun(SUCCESS)),
				(Name::new("child2"), EndOnRun(SUCCESS)),
			]))
			.trigger_payload(RUN)
			.flush();

		on_run.get().xpect_eq(vec![
			"root".to_string(),
			"child1".to_string(),
			"child2".to_string(),
		]);
		on_result.get().xpect_eq(vec![
			("child1".to_string(), SUCCESS),
			("child2".to_string(), SUCCESS),
			("root".to_string(), SUCCESS),
		]);
	}
}
