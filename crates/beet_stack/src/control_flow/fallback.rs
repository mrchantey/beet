use crate::prelude::*;
use beet_core::prelude::*;


/// ## Errors
///
/// Errors if the entity has no children.
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
	// wow good job borrow checker
	let mut input = cx.input;
	let world = cx.tool.world();
	for child in children {
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
			.call_blocking::<(), Outcome<(), ()>>(())
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
			.call_blocking::<(), Outcome<(), ()>>(())
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
			.call_blocking::<(), Outcome<(), ()>>(())
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
			.call_blocking::<(), Outcome<(), ()>>(())
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
