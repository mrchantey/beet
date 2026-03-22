use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;





pub fn action_tool<T>(streamer: T) -> impl Bundle
where
	T: Clone + Component + ActionStreamer,
{
	(streamer, async_tool(insert_stream::<T>))
}


async fn insert_stream<T>(input: AsyncToolIn<()>) -> Result<Outcome>
where
	T: Clone + Component + ActionStreamer,
{
	// TODO somehow expose settings.. ActionTool component?
	let allow_multiple_modified = false;

	let mut streamer = input.caller.get_cloned::<T>().await?;
	let mut stream = streamer.stream_actions(input.caller.clone()).await?;
	while let Some(changes) = stream.next().await {
		let meta_builder = stream.meta_builder()?;
		let ActionChanges { created, modified } = changes?;
		input
			.caller
			.with_state::<(Commands, Query<&mut Action>), _>(
				move |agent, (mut commands, mut query)| -> Result {
					// let view = query.view(entity)?;

					let mut modified = modified
						.into_iter()
						.map(|modified| (modified.id(), modified))
						.collect::<HashMap<_, _>>();

					for mut action in query.iter_mut() {
						if allow_multiple_modified
							&& let Some(new) =
								modified.get(&action.id()).cloned()
						{
							// clone if allow multiple
							action.set_if_neq(new);
						} else if let Some(new) = modified.remove(&action.id())
						{
							action.set_if_neq(new);
						}
					}
					for created_action in created {
						let meta = meta_builder.build(created_action.id());
						commands
							.entity(agent)
							.with_child((created_action, meta));
					}

					Ok(())
				},
			)
			.await?;

		// stream.write(store.inner(), changes.all_actions()).await?;
	}
	println!("Done!");

	// todo!("spawn/update action entities");

	Ok(Pass(()))
}
