use crate::prelude::*;
use beet_core::prelude::*;

/// Parallel control-flow component.
///
/// Runs all child actions concurrently with a clone of the same input.
/// Returns the first [`Outcome::Fail`] if any child fails, otherwise
/// [`Outcome::Pass`] with the original input once all children pass.
///
/// The first fail or error drops the children still running, which stops each
/// at its next await (a dropped call cancels its callee), their callers never
/// reading what they would have answered. [`Race`] resolves with whichever
/// child finishes first instead.
///
/// Child error handling is controlled by [`BypassErrors`].
///
/// ## Errors
///
/// Errors depending on [`ChildError`] flags when a child has:
/// - no [`ActionMeta`]
/// - incompatible [`ActionMeta`] signature
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn Parallel<Input = (), Output = ()>(
	cx: ActionContext<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	let children =
		BehaviourChildren::valid_for::<Input, Outcome<Input, Output>>(
			&cx.world(),
			cx.id(),
			"parallel",
		)
		.await?;

	let world = cx.world().clone();
	let input = cx.input;

	// resolved up-front by the shared query, so they all run concurrently; a
	// fail short-circuits the join as an `Err` beside a child's error, so
	// either drops the rest
	let calls = children.into_iter().map(|child| {
		let world = world.clone();
		let input = input.clone();
		async move {
			match world
				.entity(child)
				.call::<Input, Outcome<Input, Output>>(input)
				.await
			{
				Ok(Outcome::Pass(_)) => Ok(()),
				Ok(fail) => Err(Ok(fail)),
				Err(err) => Err(Err(err)),
			}
		}
	});

	match async_ext::try_join_all(calls).await {
		Ok(_) => Ok(Outcome::Pass(input)),
		Err(stopped) => stopped,
	}
}

impl Parallel {
	/// Create a default `Parallel<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

#[cfg(test)]
mod tests {
	use super::*;

	fn outcome_fail() -> Action<(), Outcome<(), ()>> {
		Action::new_pure(|_: ActionContext| Outcome::Fail(()).xok())
	}
	fn outcome_pass() -> Action<(), Outcome<(), ()>> {
		Action::new_pure(|_: ActionContext| Outcome::Pass(()).xok())
	}
	fn wrong_signature_action() -> Action<(), i32> {
		Action::new_pure(|_: ActionContext| 7.xok())
	}

	#[beet_core::test]
	async fn no_children() {
		AsyncPlugin::world()
			.spawn(Parallel::new())
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn failing_child() {
		AsyncPlugin::world()
			.spawn((Parallel::new(), children![outcome_fail()]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail(()));
	}

	#[beet_core::test]
	async fn all_passing_children() {
		AsyncPlugin::world()
			.spawn((Parallel::new(), children![
				outcome_pass(),
				outcome_pass(),
				outcome_pass(),
			]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn one_failing_child_fails() {
		AsyncPlugin::world()
			.spawn((Parallel::new(), children![
				outcome_pass(),
				outcome_fail(),
				outcome_pass(),
			]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail(()));
	}

	/// The first fail drops the siblings still running.
	#[beet_core::test]
	async fn a_fail_stops_its_siblings() {
		let mut world = AsyncPlugin::world();
		let slow = Store::<bool>::default();
		let parallel = world
			.spawn((Parallel::new(), children![
				crate::actions::test_support::sleep_then_set(30, slow.clone()),
				outcome_fail(),
			]))
			.id();
		world
			.entity_mut(parallel)
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail(()));
		crate::actions::test_support::settle(&mut world, 80).await;
		slow.get().xpect_false();
	}

	#[beet_core::test]
	async fn bypass_action_mismatch_ignores_wrong_signature() {
		AsyncPlugin::world()
			.spawn((
				Parallel::new(),
				BypassErrors(ChildError::ACTION_MISMATCH),
				children![wrong_signature_action(), outcome_pass()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn bypass_no_action_ignores_missing() {
		AsyncPlugin::world()
			.spawn((
				Parallel::new(),
				BypassErrors(ChildError::NO_ACTION),
				children![(), outcome_pass()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}
}
