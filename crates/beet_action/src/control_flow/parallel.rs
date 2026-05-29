use crate::prelude::*;
use beet_core::prelude::*;

/// Parallel control-flow component.
///
/// Runs all child actions concurrently with a clone of the same input.
/// Returns the first [`Outcome::Fail`] if any child fails, otherwise
/// [`Outcome::Pass`] with the original input once all children pass.
///
/// Children are awaited together; cancellation of in-flight siblings on
/// the first failure is deferred to the async runtime overhaul.
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[require(ParallelAction<Input,Output>)]
#[reflect(Component, Default)]
pub struct Parallel<Input = (), Output = ()>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output)>,
}

impl<Input, Output> Default for Parallel<Input, Output>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl Parallel {
	/// Create a default `Parallel<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

/// Runs all children concurrently, failing fast if any child fails.
///
/// Child error handling is controlled by [`ExcludeErrors`].
///
/// ## Errors
///
/// Errors depending on [`ChildError`] flags when a child has:
/// - no [`ActionMeta`]
/// - incompatible [`ActionMeta`] signature
#[action(default)]
#[derive(Component)]
pub async fn ParallelAction<Input, Output>(
	cx: ActionContext<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	let exclude_errors = cx
		.caller
		.get_cloned::<ExcludeErrors>()
		.await
		.unwrap_or_default();

	let children =
		match cx.caller.get(|children: &Children| children.to_vec()).await {
			Ok(children) => children,
			Err(_) => {
				// entity has no children, pass returning the input
				return Ok(Outcome::Pass(cx.input));
			}
		};

	let world = cx.world().clone();
	let input = cx.input;

	// resolve valid children up-front, then run them concurrently
	let mut calls = Vec::new();
	for child in children {
		let action_meta_result =
			world.entity(child).get(|meta: &ActionMeta| *meta).await;

		let action_meta = match action_meta_result {
			Ok(action_meta) => action_meta,
			Err(child_error) => {
				if exclude_errors.contains(ChildError::NO_ACTION) {
					continue;
				}
				bevybail!(
					"parallel child has no action: {child:?}, error: {child_error}"
				);
			}
		};

		if let Err(mismatch_error) =
			action_meta.assert_match::<Input, Outcome<Input, Output>>()
		{
			if exclude_errors.contains(ChildError::ACTION_MISMATCH) {
				continue;
			}
			bevybail!(
				"parallel child wrong action signature: {child:?}, error: {mismatch_error}"
			);
		}

		let world = world.clone();
		let input = input.clone();
		calls.push(async move {
			world
				.entity(child)
				.call::<Input, Outcome<Input, Output>>(input)
				.await
		});
	}

	for outcome in try_join_all(calls).await? {
		if let Outcome::Fail(output) = outcome {
			return Ok(Outcome::Fail(output));
		}
	}

	Ok(Outcome::Pass(input))
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

	#[beet_core::test]
	async fn exclude_action_mismatch_ignores_wrong_signature() {
		AsyncPlugin::world()
			.spawn((
				Parallel::new(),
				ExcludeErrors(ChildError::ACTION_MISMATCH),
				children![wrong_signature_action(), outcome_pass()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn exclude_no_action_ignores_missing() {
		AsyncPlugin::world()
			.spawn((
				Parallel::new(),
				ExcludeErrors(ChildError::NO_ACTION),
				children![(), outcome_pass()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}
}
