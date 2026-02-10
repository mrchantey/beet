use crate::prelude::*;
use beet_core::prelude::*;

/// A control flow return type that can be used to implement
/// fallback/sequence, if/else,
/// switch, and other control flow structures.
pub enum Outcome<P, F> {
	Pass(P),
	Fail(F),
}



/// ## Errors
///
/// Errors if the entity has no children.
pub async fn fallback<Input, Output>(
	In(ToolContext { tool, input }): In<ToolContext<Input>>,
	world: AsyncWorld,
) -> Result<Outcome<Output, Input>>
where
	Input: 'static + Send + Sync,
	Output: 'static + Send + Sync,
{
	let entity = world.entity(tool);

	let children =
		match entity.get(|children: &Children| children.to_vec()).await {
			Ok(children) => children,
			Err(_) => {
				// entity has no children, fail returning the input
				return Ok(Outcome::Fail(input));
			}
		};

	// try each child in order, returning the first pass or the last fail
	// wow good job borrow checker
	let mut input = input;
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
