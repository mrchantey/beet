use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;

/// often added alongside streamers, ie `O11sStreamer`
pub async fn post_streamer_tool<T>(cx: ToolContext<()>) -> Result<Outcome>
where
	T: Clone + Component + PostStreamer,
{
	let streamer = cx.caller.get_cloned::<T>().await?;
	post_streamer_tool_stateful(ToolContext {
		caller: cx.caller,
		input: streamer,
	})
	.await
}
pub async fn post_streamer_tool_stateful<T>(
	cx: ToolContext<T>,
) -> Result<Outcome>
where
	T: Clone + Component + PostStreamer,
{
	// TODO somehow expose settings.. PostTool component?
	let allow_multiple_modified = false;

	let streamer = cx.input;
	let mut stream = streamer.stream_posts(cx.caller.clone()).await?;
	let mut function_calls = HashMap::new();

	while let Some(changes) = stream.next().await {
		let changes = changes?;
		for post in changes
			.iter_all()
			.filter(|post| post.status() == PostStatus::Completed)
		// .map(|post| post.as_agent_post())
		{
			info!("Received post changes: {:#?}", post);

			match AgentPost::new(post) {
				AgentPost::FunctionCall(view) => {
					function_calls.insert(view.id(), view.into_owned());
				}
				_ => {}
			}
		}
		// info!("Received post changes: {:#?}", changes);
		let meta_builder = stream.meta_builder()?;
		let PostChanges { created, modified } = changes;
		cx.caller
			.with_state::<(Commands, Query<&mut Post>), _>(
				move |agent, (mut commands, mut query)| -> Result {
					// let view = query.view(entity)?;

					let mut modified = modified
						.into_iter()
						.map(|modified| (modified.id(), modified))
						.collect::<HashMap<_, _>>();

					// 1. apply modified posts
					for mut post in query.iter_mut() {
						if allow_multiple_modified
							&& let Some(new) = modified.get(&post.id()).cloned()
						{
							// clone if allow multiple
							post.set_if_neq(new);
						} else if let Some(new) = modified.remove(&post.id()) {
							post.set_if_neq(new);
						}
					}
					// 2. spawn created posts
					for created_post in created {
						let meta = meta_builder.build(created_post.id());
						commands.spawn((ChildOf(agent), created_post, meta));
					}

					Ok(())
				},
			)
			.await?;
	}

	call_functions(cx.caller, function_calls.into_values()).await?;

	Ok(Pass(()))
}
