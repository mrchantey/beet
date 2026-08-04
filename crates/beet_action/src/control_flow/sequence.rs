use crate::prelude::*;
use alloc::format;
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

/// Skips child entities whose [`ActionMeta`] is missing or does not
/// [`serve`](ActionMeta::serves) the expected
/// `Action<Input, Outcome<Input, Output>>` signature.
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
	cx.world()
		.run_system_cached_with(
			collect_valid_children::<Input, Output>,
			cx.id(),
		)
		.await?
}

fn collect_valid_children<Input, Output>(
	In(caller): In<Entity>,
	excludes: Query<&ExcludeErrors>,
	children: Query<&Children>,
	metas: Query<&ActionMeta>,
) -> Result<Vec<Entity>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let exclude_errors = excludes.get(caller).cloned().unwrap_or_default();
	let Ok(children) = children.get(caller) else {
		return Ok(Vec::new());
	};
	let mut valid = Vec::with_capacity(children.len());
	for child in children.iter() {
		let Ok(meta) = metas.get(child) else {
			if exclude_errors.contains(ChildError::NO_ACTION) {
				continue;
			}
			bevybail!("sequence child has no action: {child:?}");
		};
		if !meta.serves::<Input, Outcome<Input, Output>>() {
			if exclude_errors.contains(ChildError::ACTION_MISMATCH) {
				continue;
			}
			bevybail!(
				"sequence child wrong action signature: {child:?}, serves: {}",
				meta.signatures()
			);
		}
		valid.push(child);
	}
	Ok(valid)
}

/// Selects the first child whose [`ActionMeta`] [`serves`](ActionMeta::serves)
/// `(Input, Out)`, the downward dispatch hop: a parent hands its call to the
/// first child that can take it, ignoring config-only and differently-shaped
/// children.
///
/// # Errors
/// Errors when no child serves the signature, listing each child's signatures.
pub fn first_serving_child<Input, Out>(
	In(parent): In<Entity>,
	children: Query<&Children>,
	metas: Query<&ActionMeta>,
) -> Result<Entity>
where
	Input: 'static,
	Out: 'static,
{
	let children = children
		.get(parent)
		.map(Children::iter)
		.into_iter()
		.flatten();
	let mut candidates = Vec::new();
	for child in children {
		match metas.get(child) {
			Ok(meta) if meta.serves::<Input, Out>() => return Ok(child),
			Ok(meta) => {
				candidates.push(format!("\n  {child}: {}", meta.signatures()))
			}
			Err(_) => candidates.push(format!("\n  {child}: no action")),
		}
	}
	bevybail!(
		"no child of {parent} serves Action<{}, {}>.{}",
		core::any::type_name::<Input>(),
		core::any::type_name::<Out>(),
		candidates.concat()
	)
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

	#[beet_core::test]
	fn selects_the_first_serving_child() {
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
				first_serving_child::<(), Outcome<(), ()>>,
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
				first_serving_child::<(), Outcome<(), ()>>,
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
