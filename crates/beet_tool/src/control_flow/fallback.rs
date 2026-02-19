use crate::prelude::*;
use beet_core::prelude::*;


/// ## Errors
///
/// Errors if the entity has no children.
///
/// Children whose [`ToolMeta`] input/output types don't match
/// `Input`/`Outcome<Output, Input>` are silently skipped, preventing
/// unrelated tools (like render tools) from being called with the
/// wrong types.
pub async fn fallback<Input, Output>(
	cx: AsyncToolIn<Input>,
) -> Result<Outcome<Output, Input>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let children =
		match cx.tool.get(|children: &Children| children.to_vec()).await {
			Ok(children) => children,
			Err(_) => {
				// entity has no children, fail returning the input
				return Ok(Outcome::Fail(cx.input));
			}
		};

	// try each child in order, returning the first pass or the last fail
	let mut input = cx.input;
	let world = cx.tool.world();
	for child in children {
		// skip children whose ToolMeta doesn't match our types
		let is_compatible = world
			.entity(child)
			.get(|meta: &ToolMeta| {
				meta.assert_match::<Input, Outcome<Output, Input>>().is_ok()
			})
			.await
			.unwrap_or(false);

		if !is_compatible {
			continue;
		}

		match world
			.entity(child)
			.call::<Input, Outcome<Output, Input>>(input)
			.await?
		{
			Outcome::Pass(output) => return Ok(Outcome::Pass(output)),
			Outcome::Fail(prev_input) => {
				// try again with the returned input
				input = prev_input;
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

	#[test]
	fn no_children() {
		AsyncPlugin::world()
			.spawn(async_tool(fallback::<(), ()>))
			.call_blocking::<(), Outcome>(())
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}
	#[test]
	fn failing_child() {
		AsyncPlugin::world()
			.spawn((async_tool(fallback::<(), ()>), children![(
				PathPartial::new("foo"),
				outcome_fail(),
			)]))
			.call_blocking::<(), Outcome>(())
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}
	#[test]
	fn passing_child() {
		AsyncPlugin::world()
			.spawn((async_tool(fallback::<(), ()>), children![(
				PathPartial::new("foo"),
				outcome_pass(),
			)]))
			.call_blocking::<(), Outcome>(())
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
	#[test]
	fn passing_nth_child() {
		AsyncPlugin::world()
			.spawn((async_tool(fallback::<(), ()>), children![
				(PathPartial::new("foo"), outcome_fail()),
				(PathPartial::new("bar"), outcome_fail()),
				(PathPartial::new("bazz"), outcome_pass()),
				(PathPartial::new("boo"), outcome_fail()),
			]))
			.call_blocking::<(), Outcome>(())
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
