use crate::prelude::*;
use beet_core::prelude::*;

/// Sequence control-flow component.
///
/// Runs child actions in order, threading `Input` through each child.
/// Returns the first [`Outcome::Fail`] immediately, or [`Outcome::Pass`]
/// with the final input if all children pass.
///
/// Unlike [`Parallel`] or [`Repeat`], a sequence threads its input by move
/// and so does **not** require `Input: Clone`. For a variant that always
/// passes regardless of child results see [`InfallibleSequence`].
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[require(SequenceAction<Input,Output>)]
#[reflect(Component, Default)]
pub struct Sequence<Input = (), Output = ()>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output)>,
}

impl<Input, Output> Default for Sequence<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl Sequence {
	/// Create a default `Sequence<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

/// Skips child entities whose [`ActionMeta`] is missing or does not match
/// the expected `Action<Input, Outcome<Input, Output>>` signature.
///
/// Honours [`ExcludeErrors`]: when a flagged error is excluded the child is
/// dropped from the returned list, otherwise the error is propagated.
async fn valid_children<Input, Output>(
	cx: &ActionContext<Input>,
) -> Result<Vec<Entity>>
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
			Err(_) => return Ok(Vec::new()),
		};

	let world = cx.world();
	let mut valid = Vec::with_capacity(children.len());
	for child in children {
		let action_meta = match world
			.entity(child)
			.get(|meta: &ActionMeta| *meta)
			.await
		{
			Ok(action_meta) => action_meta,
			Err(child_error) => {
				if exclude_errors.contains(ChildError::NO_ACTION) {
					continue;
				}
				bevybail!(
					"sequence child has no action: {child:?}, error: {child_error}"
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
				"sequence child wrong action signature: {child:?}, error: {mismatch_error}"
			);
		}
		valid.push(child);
	}
	Ok(valid)
}

/// Runs children in order, returning the first [`Outcome::Fail`] immediately.
/// Returns [`Outcome::Pass`] only if all compatible children pass.
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
pub async fn SequenceAction<Input, Output>(
	cx: ActionContext<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let children = valid_children::<Input, Output>(&cx).await?;
	let world = cx.world();
	let mut input = cx.input;

	for child in children {
		match world
			.entity(child)
			.call::<Input, Outcome<Input, Output>>(input)
			.await?
		{
			Outcome::Pass(next_input) => input = next_input,
			Outcome::Fail(output) => return Ok(Outcome::Fail(output)),
		}
	}

	Ok(Outcome::Pass(input))
}

/// Sequence variant that always [`Outcome::Pass`]es.
///
/// Runs every child in order with a clone of the original input, ignoring
/// child failures, then returns [`Outcome::Pass`] with that input. Because
/// each child receives the same input it requires `Input: Clone`; for the
/// threading, fail-fast variant use [`Sequence`].
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[require(InfallibleSequenceAction<Input,Output>)]
#[reflect(Component, Default)]
pub struct InfallibleSequence<Input = (), Output = ()>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output)>,
}

impl<Input, Output> Default for InfallibleSequence<Input, Output>
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

impl InfallibleSequence {
	/// Create a default `InfallibleSequence<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

/// Runs every child once, ignoring failures, then passes with the input.
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
pub async fn InfallibleSequenceAction<Input, Output>(
	cx: ActionContext<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	let children = valid_children::<Input, Output>(&cx).await?;
	let world = cx.world();
	let input = cx.input;

	for child in children {
		// run for side effects, discarding the child's outcome
		let _ = world
			.entity(child)
			.call::<Input, Outcome<Input, Output>>(input.clone())
			.await?;
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
			.spawn(Sequence::new())
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn failing_child() {
		AsyncPlugin::world()
			.spawn((Sequence::new(), children![outcome_fail()]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail(()));
	}

	#[beet_core::test]
	async fn passing_child() {
		AsyncPlugin::world()
			.spawn((Sequence::new(), children![outcome_pass()]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn failing_nth_child() {
		AsyncPlugin::world()
			.spawn((Sequence::new(), children![
				outcome_pass(),
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
	async fn all_passing_children() {
		AsyncPlugin::world()
			.spawn((Sequence::new(), children![
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
	async fn threads_input_without_clone() {
		// a non-Clone payload proves Sequence threads by move
		struct NoClone(i32);
		fn increment()
		-> Action<NoClone, Outcome<NoClone, ()>> {
			Action::new_pure(|cx: ActionContext<NoClone>| {
				Outcome::Pass(NoClone(cx.input.0 + 1))
			})
		}
		AsyncPlugin::world()
			.spawn((Sequence::<NoClone, ()>::default(), children![
				increment(),
				increment(),
			]))
			.call::<NoClone, Outcome<NoClone, ()>>(NoClone(40))
			.await
			.unwrap()
			.xmap(|out| match out {
				Outcome::Pass(NoClone(value)) => value,
				Outcome::Fail(_) => unreachable!(),
			})
			.xpect_eq(42);
	}

	#[beet_core::test]
	async fn exclude_action_mismatch_ignores_wrong_signature() {
		AsyncPlugin::world()
			.spawn((
				Sequence::new(),
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
				Sequence::new(),
				ExcludeErrors(ChildError::NO_ACTION),
				children![(), outcome_pass()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn infallible_passes_despite_failures() {
		AsyncPlugin::world()
			.spawn((InfallibleSequence::new(), children![
				outcome_pass(),
				outcome_fail(),
				outcome_pass(),
			]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn infallible_no_children() {
		AsyncPlugin::world()
			.spawn(InfallibleSequence::new())
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}
}
