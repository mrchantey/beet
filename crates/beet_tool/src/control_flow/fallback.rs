use crate::prelude::*;
use beet_core::prelude::*;

/// Fallback control-flow component.
///
/// Runs child tools in order until one passes.
/// Returns the first [`Outcome::Pass`] immediately, otherwise returns
/// [`Outcome::Fail`] with the latest input after all children are tried.
#[derive(Debug, Component)]
#[require(Tool<Input, Outcome<Output, Input>> = async_tool(fallback_tool::<Input, Output>))]
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
	/// Set the excluded child errors.
	pub fn with_exclude_errors(mut self, exclude_errors: ChildError) -> Self {
		self.exclude_errors = exclude_errors;
		self
	}

	/// Get the current excluded child errors.
	pub fn exclude_errors(&self) -> ChildError { self.exclude_errors }
	/// Try children in order, returning the first pass or final fail.
	///
	/// Child error handling is controlled by [`Fallback::exclude_errors`].
	///
	/// ## Errors
	///
	/// Errors depending on [`ChildError`] exclusions when a child has:
	/// - no [`ToolMeta`]
	/// - incompatible [`ToolMeta`] signature
	pub async fn run(
		&self,
		cx: ToolContext<Input>,
	) -> Result<Outcome<Output, Input>>
	where
		Input: 'static + Send + Sync,
		Output: 'static + Send + Sync,
	{
		let children = match cx
			.caller
			.get(|children: &Children| children.to_vec())
			.await
		{
			Ok(children) => children,
			Err(_) => {
				// entity has no children, fail returning the input
				return Ok(Outcome::Fail(cx.input));
			}
		};

		let world = cx.world().clone();
		let mut input = cx.input;

		for child in children {
			let tool_meta_result =
				world.entity(child).get(|meta: &ToolMeta| *meta).await;

			let tool_meta = match tool_meta_result {
				Ok(tool_meta) => tool_meta,
				Err(child_error) => {
					if self.exclude_errors.contains(ChildError::NO_TOOL) {
						continue;
					}
					bevybail!(
						"fallback child has no tool: {child:?}, error: {child_error}"
					);
				}
			};

			if let Err(mismatch_error) =
				tool_meta.assert_match::<Input, Outcome<Output, Input>>()
			{
				if self.exclude_errors.contains(ChildError::TOOL_MISMATCH) {
					continue;
				}
				bevybail!(
					"fallback child has incorrect tool signature: {child:?}, error: {mismatch_error}"
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
}

async fn fallback_tool<Input, Output>(
	cx: ToolContext<Input>,
) -> Result<Outcome<Output, Input>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	cx.caller
		.get_cloned::<Fallback<Input, Output>>()
		.await
		.unwrap_or_default()
		.run(cx)
		.await
}

#[cfg(test)]
mod tests {
	use super::*;

	fn outcome_fail() -> Tool<(), Outcome> {
		func_tool(|_: ToolContext| Outcome::FAIL.xok())
	}
	fn outcome_pass() -> Tool<(), Outcome> {
		func_tool(|_: ToolContext| Outcome::PASS.xok())
	}
	fn wrong_signature_tool() -> Tool<(), i32> {
		func_tool(|_: ToolContext| 7.xok())
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
	async fn exclude_no_tool_ignores_missing() {
		AsyncPlugin::world()
			.spawn((
				Fallback::new().with_exclude_errors(ChildError::NO_TOOL),
				children![(), outcome_pass()],
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn exclude_tool_mismatch_ignores_wrong_signature() {
		AsyncPlugin::world()
			.spawn((
				Fallback::new().with_exclude_errors(ChildError::TOOL_MISMATCH),
				children![wrong_signature_tool(), outcome_pass()],
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
