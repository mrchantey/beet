use crate::prelude::*;
use beet_core::prelude::*;

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
/// 		EndWith(Outcome::Pass),
/// 		EndWith(Outcome::Pass),
/// 	]))
/// 	.trigger_target(GetOutcome)
/// 	.flush();
/// ```
#[action(on_start, on_next)]
#[derive(Default, Component, Deref, DerefMut, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct Parallel(pub HashSet<Entity>);

fn on_start(
	mut ev: On<GetOutcome>,
	mut commands: Commands,
	mut query: Query<(&mut Parallel, &Children)>,
) -> Result {
	let (mut action, children) = query.get_mut(ev.action())?;
	action.clear();

	if children.is_empty() {
		commands
			.entity(ev.action())
			.trigger_target(Outcome::Pass);
		return Ok(());
	}

	for child in children.iter() {
		ev.trigger_next_with(child, GetOutcome);
	}
	Ok(())
}

fn on_next(
	mut ev: On<ChildEnd<Outcome>>,
	mut query: Query<(&mut Parallel, &Children)>,
) -> Result {
	let target = ev.action();
	let child = ev.child();

	// if any error, just propagate the error
	if ev.is_fail() {
		ev.propagate_child();
		return Ok(());
	}

	let (mut action, children) = query.get_mut(target)?;
	action.insert(child);

	// if all children have completed successfully, succeed
	if action.len() == children.len() {
		ev.propagate_child();
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

	#[test]
	fn succeeds() {
		let mut world = BeetFlowPlugin::world();

		let on_result = collect_on_result(&mut world);
		let on_run = collect_on_run(&mut world);

		world
			.spawn((Name::new("root"), Parallel::default(), children![
				(Name::new("child1"), EndWith(Outcome::Pass)),
				(Name::new("child2"), EndWith(Outcome::Pass)),
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
			("child2".to_string(), Outcome::Pass),
			("root".to_string(), Outcome::Pass),
		]);
	}
}
