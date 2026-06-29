use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// often added alongside streamers, ie `O11sStreamer`
pub async fn post_streamer_action<T>(cx: ActionContext) -> Result<Outcome>
where
	T: Clone + Component + PostStreamer,
{
	let streamer = cx.caller.get_cloned::<T>().await?;
	post_streamer_action_stateful(ActionContext {
		caller: cx.caller,
		input: streamer,
	})
	.await
}

/// Stream a model response into the thread's [`ThreadWindow`], surfacing a
/// failure as an error post rather than a swallowed error.
///
/// On any streaming failure (missing/invalid key, network error, or a mid-stream
/// error event) the message is appended to the window as a 5xx error post, so it
/// renders in the thread view as an `Error` node (TUI mode hides stderr), and the
/// turn [`Pass`]es so a finite program still completes instead of hanging.
pub async fn post_streamer_action_stateful<T>(
	cx: ActionContext<T>,
) -> Result<Outcome>
where
	T: Clone + Component + PostStreamer,
{
	let caller = cx.caller.clone();
	if let Err(err) = stream_into_window(cx).await {
		let message = err.to_string();
		// also log for headless/non-TUI runs where the thread view isn't visible
		error!("model streaming failed: {message}");
		let (actor_id, thread_id) = caller
			.with_state::<ThreadWindowQuery, _>(
				|entity, window| -> Result<(ActorId, ThreadId)> {
					Ok((window.actor_id(entity)?, window.thread_id(entity)?))
				},
			)
			.await??;
		caller
			.with_state::<ThreadWindowQuery, _>(
				move |entity, mut window| -> Result {
					window.push_post(
						entity,
						AgentPost::new_error(
							actor_id,
							thread_id,
							message,
							PostStatus::Completed,
						),
					)
				},
			)
			.await??;
	}
	Ok(Pass(()))
}

/// The streaming core: drive the model response into `window.posts` (the live
/// slice the scene renders and persists), rather than spawning per-post entities.
/// Completed function-call posts are collected and dispatched after the stream
/// finishes. Errors propagate to [`post_streamer_action_stateful`], which renders
/// them as an error post.
async fn stream_into_window<T>(cx: ActionContext<T>) -> Result
where
	T: Clone + Component + PostStreamer,
{
	let streamer = cx.input;
	let mut stream = streamer.stream_posts(cx.caller.clone()).await?;
	let mut function_calls = HashMap::new();

	while let Some(changes) = stream.next().await {
		let changes = changes?;

		// collect completed function calls to dispatch after the stream
		for post in changes
			.iter_all()
			.filter(|post| post.status() == PostStatus::Completed)
		{
			if let AgentPost::FunctionCall(view) = AgentPost::new(post) {
				function_calls.insert(view.id(), view.into_owned());
			}
		}

		// build response metas only once posts have been created
		let PostChanges { created, modified } = changes;
		let metas = if created.is_empty() {
			Vec::new()
		} else {
			let meta_builder = stream.meta_builder()?;
			created
				.iter()
				.map(|post| meta_builder.build(post.id()))
				.collect::<Vec<_>>()
		};

		// apply the changes to the thread window
		cx.caller
			.with_state::<ThreadWindowQuery, _>(
				move |entity, mut window_mut| -> Result {
					let mut window = window_mut.window_mut(entity)?;
					modified
						.into_iter()
						.for_each(|post| window.upsert_post(post));
					created.into_iter().zip(metas).for_each(|(post, meta)| {
						window.set_meta(meta);
						window.upsert_post(post);
					});
					Ok(())
				},
			)
			.await??;
	}

	call_functions(cx.caller, function_calls.into_values()).await?;

	Ok(())
}
