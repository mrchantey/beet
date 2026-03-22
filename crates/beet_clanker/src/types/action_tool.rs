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
	let entity = input.caller.id();
	let (store, mut streamer, actor, thread) = input
		.caller
		.world()
		.run_system_cached_with(get_cx::<T>, entity)
		.await?;
	let mut stream = streamer
		.stream_actions(store.inner(), actor, thread)
		.await?;

	// let mut threads = DocMap::<Thread>::new();
	// let mut actors = DocMap::<Actor>::new();
	// let get_or_create_actor = async |id:ActorId|->Result<Actor>{
	// if let Some(actor) = actors.get(id) {
	// 	Ok(actor.clone())
	// } else {
	// 	let actor = store.insert_actor(actor)
	// 	actors.insert(id, actor.clone());
	// 	Ok(actor)
	// };

	while let Some(ev) = stream.next().await {
		let changes = ev?;
		// for created in changes.created.into_iter() {
		// 	let action = stream.actions().get(created)?;
		// 	trace!("action cr: {changes:#?}");
		// 	// let thread = threads.get(change
		// }

		stream.write(store.inner(), changes.all_actions()).await?;
	}


	Ok(Pass(()))
}


fn get_cx<T: ActionStreamer + Component + Clone>(
	entity: In<Entity>,
	query: Query<(&ActionStore, &T, &ActorId, &ThreadId)>,
) -> Result<(ActionStore, T, ActorId, ThreadId)> {
	let (store, streamer, action, thread) = query.get(*entity)?;
	Ok((
		store.clone(),
		streamer.clone(),
		action.clone(),
		thread.clone(),
	))
}
