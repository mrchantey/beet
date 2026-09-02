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

/// The steps of this call's caller that can serve it.
///
/// One line each way to [`BehaviourChildren`], which owns both rules: markup
/// punctuation is not a step at all, and a step the [`BypassErrors`] policy
/// does not excuse is a failure.
async fn valid_children<Input, Output>(
	cx: &ActionContext<Input>,
) -> Result<Vec<Entity>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	BehaviourChildren::valid_for::<Input, Outcome<Input, Output>>(
		&cx.world(),
		cx.id(),
		"sequence",
	)
	.await
}

/// Runs children in order, returning the first [`Outcome::Fail`] immediately.
/// Returns [`Outcome::Pass`] only if all compatible children pass.
///
/// Child error handling is controlled by [`BypassErrors`].
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
/// Child error handling is controlled by [`BypassErrors`].
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
		fn increment() -> Action<NoClone, Outcome<NoClone, ()>> {
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
	async fn bypass_action_mismatch_ignores_wrong_signature() {
		AsyncPlugin::world()
			.spawn((
				Sequence::new(),
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
				Sequence::new(),
				BypassErrors(ChildError::NO_ACTION),
				children![(), outcome_pass()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	/// A child whose canonical action is the wrong shape still serves the
	/// sequence when an [`ActionOverload`] adapts it.
	#[beet_core::test]
	async fn overloaded_child() {
		AsyncPlugin::world()
			.spawn((Sequence::new(), children![(
				wrong_signature_action(),
				ActionOverload::new(Action::<(), Outcome<(), ()>>::new_pure(
					|_: ActionContext| Outcome::Pass(())
				))
			)]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	/// A sequence that skipped every child fails rather than passing, so a run
	/// that did nothing is never a clean exit.
	#[beet_core::test]
	async fn all_children_skipped_errors() {
		AsyncPlugin::world()
			.spawn((
				Sequence::new(),
				BypassErrors(ChildError::NO_ACTION),
				children![()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("skipped all 1 of its children");
	}

	/// ..unless the caller declares that running nothing is a valid outcome.
	#[beet_core::test]
	async fn none_valid_is_bypassable() {
		AsyncPlugin::world()
			.spawn((
				Sequence::new(),
				BypassErrors(ChildError::all()),
				children![()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	fn selects_the_first_matching_child() {
		let mut world = World::new();
		let parent = world
			.spawn(children![
				Name::new("config-only"),
				wrong_signature_action(),
				outcome_pass(),
				outcome_fail(),
			])
			.id();
		world.flush();
		let serving = world
			.run_system_cached_with::<_, Result<Entity>, _, _>(
				first_matching_child::<(), Outcome<(), ()>>,
				parent,
			)
			.unwrap()
			.unwrap();
		world.entity(serving).get::<ActionMeta>().xpect_some();
		// the two skipped children come first, so the pass action is the third
		serving.xpect_eq(world.entity(parent).get::<Children>().unwrap()[2]);
	}

	#[beet_core::test]
	fn no_serving_child_lists_signatures() {
		let mut world = World::new();
		let parent = world.spawn(children![wrong_signature_action()]).id();
		world.flush();
		world
			.run_system_cached_with::<_, Result<Entity>, _, _>(
				first_matching_child::<(), Outcome<(), ()>>,
				parent,
			)
			.unwrap()
			.unwrap_err()
			.to_string()
			.xref()
			.xpect_contains("i32");
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
