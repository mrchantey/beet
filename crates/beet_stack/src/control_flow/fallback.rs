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
	cx: AsyncToolContext<Input>,
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

	#[test]
	fn no_children() {
		StackPlugin::world()
			.spawn(tool(fallback::<(), ()>))
			.call_blocking::<(), Outcome>(())
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}
	#[test]
	fn failing_child() {
		StackPlugin::world()
			.spawn((tool(fallback::<(), ()>), children![(
				PathPartial::new("foo"),
				tool(|| { Outcome::FAIL })
			)]))
			.call_blocking::<(), Outcome>(())
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}
	#[test]
	fn passing_child() {
		StackPlugin::world()
			.spawn((tool(fallback::<(), ()>), children![(
				PathPartial::new("foo"),
				tool(|| { Outcome::PASS })
			)]))
			.call_blocking::<(), Outcome>(())
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
	#[test]
	fn passing_nth_child() {
		StackPlugin::world()
			.spawn((tool(fallback::<(), ()>), children![
				(PathPartial::new("foo"), tool(|| { Outcome::FAIL }),),
				(PathPartial::new("bar"), tool(|| { Outcome::FAIL }),),
				(PathPartial::new("bazz"), tool(|| { Outcome::PASS })),
				(PathPartial::new("boo"), tool(|| { Outcome::FAIL }),),
			]))
			.call_blocking::<(), Outcome>(())
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
