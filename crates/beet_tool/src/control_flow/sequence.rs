use crate::prelude::*;
use beet_core::prelude::*;



pub fn sequence<Input, Output>() -> impl Bundle
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	async_tool(sequence_tool::<Input, Output>)
}

/// Runs children in order, returning the first [`Outcome::Fail`] immediately.
/// Returns [`Outcome::Pass`] only if all compatible children pass.
///
/// This is the dual of [`fallback_tool`]: input is threaded forward through
/// [`Outcome::Pass`] arms, and the sequence short-circuits on the first
/// [`Outcome::Fail`].
///
/// ## Errors
///
/// Returns [`Outcome::Fail`] if the entity has no children.
///
/// Children whose [`ToolMeta`] input/output types don't match
/// `Input`/`Outcome<Input, Output>` are silently skipped, preventing
/// unrelated tools (like render tools) from being called with the
/// wrong types.
pub async fn sequence_tool<Input, Output>(
	cx: AsyncToolIn<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let children =
		match cx.caller.get(|children: &Children| children.to_vec()).await {
			Ok(children) => children,
			Err(_) => {
				// entity has no children, pass returning the input
				return Ok(Outcome::Pass(cx.input));
			}
		};

	// try each child in order, returning the first fail or the last pass
	let mut input = cx.input;
	let world = cx.caller.world();
	for child in children {
		// skip children whose ToolMeta doesn't match our types
		let is_compatible = world
			.entity(child)
			.get(|meta: &ToolMeta| {
				meta.assert_match::<Input, Outcome<Input, Output>>().is_ok()
			})
			.await
			.unwrap_or(false);

		if !is_compatible {
			continue;
		}

		match world
			.entity(child)
			.call::<Input, Outcome<Input, Output>>(input)
			.await?
		{
			Outcome::Pass(next_input) => {
				// thread input forward to the next child
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

	#[beet_core::test]
	async fn no_children() {
		AsyncPlugin::world()
			.spawn(sequence::<(), ()>())
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}
	#[beet_core::test]
	async fn failing_child() {
		AsyncPlugin::world()
			.spawn((sequence::<(), ()>(), children![outcome_fail()]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail(()));
	}
	#[beet_core::test]
	async fn passing_child() {
		AsyncPlugin::world()
			.spawn((sequence::<(), ()>(), children![outcome_pass()]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}
	#[beet_core::test]
	async fn failing_nth_child() {
		AsyncPlugin::world()
			.spawn((sequence::<(), ()>(), children![
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
			.spawn((sequence::<(), ()>(), children![
				outcome_pass(),
				outcome_pass(),
				outcome_pass(),
			]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}
}
