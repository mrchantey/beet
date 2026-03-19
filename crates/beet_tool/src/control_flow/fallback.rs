use crate::prelude::*;
use beet_core::prelude::*;

/// Fallback control-flow component.
///
/// Runs child tools in order until one passes.
/// Returns the first [`Outcome::Pass`] immediately, otherwise returns
/// [`Outcome::Fail`] with the latest input after all children are tried.
#[derive(Debug, Clone, Copy, Component)]
#[require(Tool<Input, Outcome<Output, Input>> = async_tool(fallback_tool::<Input, Output>))]
pub struct Fallback<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	/// Whether to propagate or ignore child-tool mismatches.
	/// Defaults to [`ChildMismatch::Any`].
	child_mismatch: ChildMismatch,
	_marker: PhantomData<fn() -> (Input, Output)>,
}

impl<Input, Output> Default for Fallback<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			child_mismatch: ChildMismatch::Any,
			_marker: PhantomData,
		}
	}
}

impl<Input, Output> Fallback<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	/// Set the child mismatch policy.
	pub fn with_child_mismatch(
		mut self,
		child_mismatch: ChildMismatch,
	) -> Self {
		self.child_mismatch = child_mismatch;
		self
	}

	/// Get the current child mismatch policy.
	pub fn child_mismatch(&self) -> ChildMismatch { self.child_mismatch }
}

/// Convenience constructor for [`Fallback`].
pub fn fallback<Input, Output>() -> Fallback<Input, Output>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	Fallback::default()
}

/// Try children in order, returning the first pass or final fail.
///
/// Child mismatch handling is controlled by [`Fallback::child_mismatch`].
///
/// ## Errors
///
/// Errors depending on [`ChildMismatch`] policy when a child has:
/// - no [`ToolMeta`]
/// - incompatible [`ToolMeta`] signature
pub async fn fallback_tool<Input, Output>(
	cx: AsyncToolIn<Input>,
) -> Result<Outcome<Output, Input>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let child_mismatch = cx
		.caller
		.get(|fallback: &Fallback<Input, Output>| fallback.child_mismatch())
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

	let mut input = cx.input;
	let world = cx.caller.world();

	for child in children {
		let tool_meta_result =
			world.entity(child).get(|meta: &ToolMeta| *meta).await;

		let tool_meta = match tool_meta_result {
			Ok(tool_meta) => tool_meta,
			Err(child_error) => match child_mismatch {
				ChildMismatch::Any | ChildMismatch::NoTool => {
					bevybail!(
						"fallback child has no tool: {child:?}, error: {child_error}"
					);
				}
				ChildMismatch::WrongTool => continue,
			},
		};

		if let Err(mismatch_error) =
			tool_meta.assert_match::<Input, Outcome<Output, Input>>()
		{
			match child_mismatch {
				ChildMismatch::Any | ChildMismatch::WrongTool => {
					bevybail!(
						"fallback child wrong tool signature: {child:?}, error: {mismatch_error}"
					);
				}
				ChildMismatch::NoTool => continue,
			}
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

	fn outcome_fail() -> Tool<(), Outcome> {
		func_tool(|_: FuncToolIn<()>| Outcome::FAIL.xok())
	}
	fn outcome_pass() -> Tool<(), Outcome> {
		func_tool(|_: FuncToolIn<()>| Outcome::PASS.xok())
	}
	fn wrong_signature_tool() -> Tool<(), i32> {
		func_tool(|_: FuncToolIn<()>| 7.xok())
	}

	#[beet_core::test]
	async fn no_children() {
		AsyncPlugin::world()
			.spawn(fallback::<(), ()>())
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn failing_child() {
		AsyncPlugin::world()
			.spawn((fallback::<(), ()>(), children![outcome_fail()]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn passing_child() {
		AsyncPlugin::world()
			.spawn((fallback::<(), ()>(), children![outcome_pass()]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn passing_nth_child() {
		AsyncPlugin::world()
			.spawn((fallback::<(), ()>(), children![
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
	async fn child_mismatch_any_with_compatible_children() {
		AsyncPlugin::world()
			.spawn((fallback::<(), ()>(), children![
				outcome_fail(),
				outcome_pass(),
			]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn child_mismatch_wrong_tool_ignores_missing_tool() {
		AsyncPlugin::world()
			.spawn((
				fallback::<(), ()>()
					.with_child_mismatch(ChildMismatch::WrongTool),
				children![(), outcome_pass()],
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn child_mismatch_no_tool_ignores_wrong_signature() {
		AsyncPlugin::world()
			.spawn((
				fallback::<(), ()>().with_child_mismatch(ChildMismatch::NoTool),
				children![wrong_signature_tool(), outcome_pass()],
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
