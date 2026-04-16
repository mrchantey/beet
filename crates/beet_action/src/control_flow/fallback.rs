use crate::prelude::*;
use beet_core::prelude::*;

/// Fallback control-flow component.
///
/// Runs child actions in order until one passes.
/// Returns the first [`Outcome::Pass`] immediately, otherwise returns
/// [`Outcome::Fail`] with the latest input after all children are tried.
#[derive(Debug, Component)]
#[require(FallbackAction<Input,Output>)]
pub struct Fallback<Input = (), Output = ()>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	/// Which child errors to skip rather than propagate.
	/// Defaults to [`ChildError::empty`].
	exclude_errors: ChildError,
	_marker: PhantomData<fn() -> (Input, Output)>,
}

impl<Input, Output> Clone for Fallback<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	fn clone(&self) -> Self {
		Self {
			exclude_errors: self.exclude_errors,
			_marker: PhantomData,
		}
	}
}
impl<Input, Output> Copy for Fallback<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
}

impl<Input, Output> Default for Fallback<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			exclude_errors: ChildError::empty(),
			_marker: PhantomData,
		}
	}
}
impl Fallback<(), ()> {
	/// Create a default `Fallback<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

impl<Input, Output> Fallback<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
}
/// Try children in order, returning the first pass or final fail.
///
/// Child error handling is controlled by [`Fallback::exclude_errors`].
///
/// ## Errors
///
/// Errors depending on [`ChildError`] exclusions when a child has:
/// - no [`ActionMeta`]
/// - incompatible [`ActionMeta`] signature
#[action(default)]
#[derive(Component)]
pub async fn FallbackAction<Input, Output>(
	cx: ActionContext<Input>,
) -> Result<Outcome<Output, Input>>
where
	Input: 'static + Send + Sync,
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
				// entity has no children, fail returning the input
				return Ok(Outcome::Fail(cx.input));
			}
		};

	let world = cx.world().clone();
	let mut input = cx.input;

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
					"fallback child has no action: {child:?}, error: {child_error}"
				);
			}
		};

		if let Err(mismatch_error) =
			action_meta.assert_match::<Input, Outcome<Output, Input>>()
		{
			if exclude_errors.contains(ChildError::ACTION_MISMATCH) {
				continue;
			}
			bevybail!(
				"fallback child has incorrect action signature: {child:?}, error: {mismatch_error}"
			);
		}

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
	async fn default_exclude_errors_with_compatible_children() {
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
	async fn exclude_no_action_ignores_missing() {
		AsyncPlugin::world()
			.spawn((
				Fallback::new(),
				ExcludeErrors(ChildError::NO_ACTION),
				children![(), outcome_pass()],
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn exclude_action_mismatch_ignores_wrong_signature() {
		AsyncPlugin::world()
			.spawn((
				Fallback::new(),
				ExcludeErrors(ChildError::ACTION_MISMATCH),
				children![wrong_signature_action(), outcome_pass()],
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
