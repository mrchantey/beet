use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;





pub fn action_tool<T>(streamer: T) -> impl Bundle
where
	T: ActionStreamer + Component,
{
	streamer
}


#[tool]
async fn stream_actions<T>(input: AsyncToolIn<()>) -> Result<Outcome>
where
	T: ActionStreamer + Component,
{
	let entity = input.caller.id();
	let world = input.caller.world();
	let streamer = input.caller.get_cloned::<T>().await?;

	Ok(Pass(()))
}
