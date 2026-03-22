use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;





pub fn action_tool<T>(streamer: T) -> impl Bundle
where
	T: Clone + Component + ActionStreamer,
{
	(streamer, async_tool(stream_actions::<T>))
}


async fn stream_actions<T>(input: AsyncToolIn<()>) -> Result<Outcome>
where
	T: Clone + Component + ActionStreamer,
{
	let mut streamer = input.caller.get_cloned::<T>().await?;
	let mut stream = streamer.stream_actions(input.caller.clone()).await?;
	while let Some(ev) = stream.next().await {
		let changes = ev?;
		// println!("action changes: {changes:#?}");
		// for created in changes.created.into_iter() {
		// 	let action = stream.actions().get(created)?;
		// 	trace!("action cr: {changes:#?}");
		// 	// let thread = threads.get(change
		// }

		// stream.write(store.inner(), changes.all_actions()).await?;
	}
	println!("Done!");

	todo!("spawn/update action entities");

	Ok(Pass(()))
}
