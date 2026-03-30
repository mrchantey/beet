use crate::prelude::*;
use beet_core::prelude::*;

/// Sequence control-flow component.
///
/// Runs child tools in order, threading `Input` through each child.
/// Returns the first [`Outcome::Fail`] immediately, or [`Outcome::Pass`]
/// with the final input if all compatible children pass.
#[derive(Debug, Clone, Copy, Component)]
#[require(Tool<Input, Outcome<Input, Output>> = async_tool(sequence_tool::<Input, Output>))]
pub struct Sequence<Input = (), Output = ()>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	/// Which child errors to skip instead of propagating.
	/// Defaults to [`ChildError::empty`].
	exclude_errors: ChildError,
	_marker: PhantomData<fn() -> (Input, Output)>,
}

impl<Input, Output> Default for Sequence<Input, Output>
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

impl<Input, Output> Sequence<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	/// Set which child errors to exclude.
	pub fn with_exclude_errors(mut self, exclude_errors: ChildError) -> Self {
		self.exclude_errors = exclude_errors;
		self
	}

	pub fn allow_no_tool(mut self) -> Self {
		self.exclude_errors |= ChildError::NO_TOOL;
		self
	}

	/// Get the current excluded errors.
	pub fn exclude_errors(&self) -> ChildError { self.exclude_errors }
}

impl Sequence {
	/// Create a default `Sequence<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

/// Runs children in order, returning the first [`Outcome::Fail`] immediately.
/// Returns [`Outcome::Pass`] only if all compatible children pass.
///
/// Child error handling is controlled by [`Sequence::exclude_errors`].
///
/// ## Errors
///
/// Errors depending on [`ChildError`] flags when a child has:
/// - no [`ToolMeta`]
/// - incompatible [`ToolMeta`] signature
async fn sequence_tool<Input, Output>(
	cx: AsyncToolIn<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let exclude_errors = cx
		.caller
		.get(|sequence: &Sequence<Input, Output>| sequence.exclude_errors())
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

	let mut input = cx.input;
	let world = cx.caller.world();

	for child in children {
		let tool_meta_result =
			world.entity(child).get(|meta: &ToolMeta| *meta).await;

		let tool_meta = match tool_meta_result {
			Ok(tool_meta) => tool_meta,
			Err(child_error) => {
				if exclude_errors.contains(ChildError::NO_TOOL) {
					continue;
				}
				bevybail!(
					"sequence child has no tool: {child:?}, error: {child_error}"
				);
			}
		};

		if let Err(mismatch_error) =
			tool_meta.assert_match::<Input, Outcome<Input, Output>>()
		{
			if exclude_errors.contains(ChildError::TOOL_MISMATCH) {
				continue;
			}
			bevybail!(
				"sequence child wrong tool signature: {child:?}, error: {mismatch_error}"
			);
		}

		match world
			.entity(child)
			.call::<Input, Outcome<Input, Output>>(input)
			.await?
		{
			Outcome::Pass(next_input) => {
				input = next_input;
			}
			Outcome::Fail(output) => return Ok(Outcome::Fail(output)),
		}
	}

	Ok(Outcome::Pass(input))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn outcome_fail() -> Tool<(), Outcome<(), ()>> {
		func_tool(|_: FuncToolIn<()>| Outcome::Fail(()).xok())
	}
	fn outcome_pass() -> Tool<(), Outcome<(), ()>> {
		func_tool(|_: FuncToolIn<()>| Outcome::Pass(()).xok())
	}
	fn wrong_signature_tool() -> Tool<(), i32> {
		func_tool(|_: FuncToolIn<()>| 7.xok())
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
	async fn default_exclude_errors_with_compatible_children() {
		AsyncPlugin::world()
			.spawn((Sequence::new(), children![outcome_pass(), outcome_pass()]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn exclude_tool_mismatch_ignores_wrong_signature() {
		AsyncPlugin::world()
			.spawn((
				Sequence::new().with_exclude_errors(ChildError::TOOL_MISMATCH),
				children![wrong_signature_tool(), outcome_pass()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn exclude_no_tool_ignores_missing() {
		AsyncPlugin::world()
			.spawn((
				Sequence::new().with_exclude_errors(ChildError::NO_TOOL),
				children![(), outcome_pass()],
			))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}
}
