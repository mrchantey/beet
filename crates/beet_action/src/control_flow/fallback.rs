use crate::prelude::*;
use beet_core::prelude::*;

/// Fallback control-flow component.
///
/// Runs child actions in order until one passes.
/// Returns the first [`Outcome::Pass`] immediately, otherwise returns
/// [`Outcome::Fail`] with the latest input after all children are tried.
///
/// Child error handling is controlled by [`BypassErrors`].
///
/// ## Errors
///
/// Errors depending on [`ChildError`] bypasses when a child has:
/// - no [`ActionMeta`]
/// - incompatible [`ActionMeta`] signature
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn Fallback<Input = (), Output = ()>(
	cx: ActionContext<Input>,
) -> Result<Outcome<Output, Input>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let children =
		BehaviourChildren::valid_for::<Input, Outcome<Output, Input>>(
			&cx.world(),
			cx.id(),
			"fallback",
		)
		.await?;

	let world = cx.world().clone();
	let mut input = cx.input;

	for child in children {
		match world
			.entity(child)
			.call::<Input, Outcome<Output, Input>>(input)
			.await?
		{
			Outcome::Pass(output) => return Ok(Outcome::Pass(output)),
			Outcome::Fail(next_input) => {
				input = next_input;
			}
		}
	}

	Ok(Outcome::Fail(input))
}

impl Fallback {
	/// Create a default `Fallback<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

#[cfg(test)]
mod tests {
	use super::*;

	fn outcome_fail() -> Action<(), Outcome> {
		Action::new_pure(|_: ActionContext| Outcome::FAIL.xok())
	}
	fn outcome_pass() -> Action<(), Outcome> {
		Action::new_pure(|_: ActionContext| Outcome::PASS.xok())
	}
	fn wrong_signature_action() -> Action<(), i32> {
		Action::new_pure(|_: ActionContext| 7.xok())
	}

	#[beet_core::test]
	async fn no_children() {
		AsyncPlugin::world()
			.spawn(Fallback::new())
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn failing_child() {
		AsyncPlugin::world()
			.spawn((Fallback::new(), children![outcome_fail()]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn passing_child() {
		AsyncPlugin::world()
			.spawn((Fallback::new(), children![outcome_pass()]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn passing_nth_child() {
		AsyncPlugin::world()
			.spawn((Fallback::new(), children![
				outcome_fail(),
				outcome_fail(),
				outcome_pass(),
				outcome_fail(),
			]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn default_bypass_errors_with_compatible_children() {
		AsyncPlugin::world()
			.spawn((
				Fallback::new(),
				children![outcome_fail(), outcome_pass(),],
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn bypass_no_action_ignores_missing() {
		AsyncPlugin::world()
			.spawn((
				Fallback::new(),
				BypassErrors(ChildError::NO_ACTION),
				children![(), outcome_pass()],
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn bypass_action_mismatch_ignores_wrong_signature() {
		AsyncPlugin::world()
			.spawn((
				Fallback::new(),
				BypassErrors(ChildError::ACTION_MISMATCH),
				children![wrong_signature_action(), outcome_pass()],
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
