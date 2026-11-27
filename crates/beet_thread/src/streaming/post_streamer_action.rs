use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use core::pin::pin;
use futures::future;
use futures::future::Either;

/// The action of a [`PostStreamer`] component, ie [`O11sStreamer`], reading it
/// off the caller: [`post_streamer_action_stateful`].
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

/// Stream a model response into the thread's [`ThreadRecords`] as the turn of
/// the agent at `cx.caller`, surfacing a failure as an error post rather than
/// a swallowed error.
///
/// On any streaming failure (missing/invalid key, network error, or a mid-stream
/// error event) the message is appended to the records as a `error` post, so it
/// renders in the thread view as an `Error` node (TUI mode hides stderr), and the
/// turn [`Pass`]es so a finite program still completes instead of hanging.
///
/// The turn honours [`InterruptTurn`]: the stream is dropped with the tool
/// calls in flight, the turn closed as interrupted and passed. A turn whose
/// call is dropped, ie the loser of a `Race`, is closed the same way.
pub async fn post_streamer_action_stateful<T>(
	cx: ActionContext<T>,
) -> Result<Outcome>
where
	T: Clone + Component + PostStreamer,
{
	let caller = cx.caller.clone();
	let (turn, interrupted) = TurnGuard::begin(caller.clone()).await?;
	let streamed = pin!(stream_into_records(cx));
	let interrupted = pin!(interrupted);
	// the losing future drops with the match, the stream and its tool calls
	// with it
	let interrupted = match future::select(streamed, interrupted).await {
		Either::Left((Ok(()), _)) => false,
		Either::Left((Err(err), _)) => {
			push_error_post(&caller, err).await?;
			false
		}
		Either::Right(_) => true,
	};
	turn.close(interrupted).await?;
	Ok(Pass(()))
}

/// Append `err` to the records as an error post by the agent at `caller`.
async fn push_error_post(caller: &AsyncEntity, err: BevyError) -> Result {
	let message = err.to_string();
	// also log for headless/non-TUI runs where the thread view isn't visible
	error!("model streaming failed: {message}");
	caller
		.with_state::<ThreadRecordsQuery, _>(
			move |entity, mut records| -> Result {
				let post = AgentPost::new_error(
					records.actor_id(entity)?,
					records.thread_id(entity)?,
					message,
					PostStatus::Completed,
				);
				records.push_post(entity, post)
			},
		)
		.await?
}

/// The streaming core: drive the model response into the window's records,
/// which its view draws, window sync applies to other windows and its store
/// persists, rather than spawning per-post entities.
/// Completed function-call posts are collected and dispatched after the stream
/// finishes. Errors propagate to [`post_streamer_action_stateful`], which renders
/// them as an error post.
async fn stream_into_records<T>(cx: ActionContext<T>) -> Result
where
	T: Clone + Component + PostStreamer,
{
	let streamer = cx.input;
	// a compacting agent refreshes or compacts its context before each call
	Compact::prepare(&cx.caller).await?;
	let mut stream = streamer.stream_posts(cx.caller.clone()).await?;
	let mut function_calls = HashMap::new();
	let mut last_created = None;

	while let Some(changes) = stream.next().await {
		let changes = changes?;

		// collect completed function calls to dispatch after the stream
		for post in changes
			.iter_all()
			.filter(|post| post.status() == PostStatus::Completed)
		{
			if let Some(AgentPost::FunctionCall(view)) = AgentPost::new(post) {
				function_calls.insert(view.id(), view.into_owned());
			}
		}

		// build response metas only once posts have been created
		let PostChanges { created, modified } = changes;
		if let Some(post) = created.last() {
			last_created = Some(post.id());
		}
		let metas = if created.is_empty() {
			Vec::new()
		} else {
			let meta_builder = stream.meta_builder()?;
			created
				.iter()
				.map(|post| meta_builder.build(post.id()))
				.collect::<Vec<_>>()
		};

		// apply the changes to the thread records
		cx.caller
			.with_state::<ThreadRecordsQuery, _>(
				move |entity, mut records_mut| -> Result {
					let mut records = records_mut.records_mut(entity)?;
					modified
						.into_iter()
						.for_each(|post| records.upsert_post(post));
					created.into_iter().zip(metas).for_each(|(post, meta)| {
						records.set_meta(meta);
						records.upsert_post(post);
					});
					Ok(())
				},
			)
			.await??;
	}

	// the response's usage lands on its last post's meta, so a total over
	// the records counts each response once; log the prompt-cache hit rate, for
	// tuning the append-only records (see perceive-act).
	if let Some(usage) = stream.token_usage().map(Usage::from) {
		let pct = if usage.input_tokens > 0 {
			usage.cached_input_tokens as f32 / usage.input_tokens as f32 * 100.0
		} else {
			0.0
		};
		info!(
			"tokens: {} in ({} cached {pct:.0}%), {} out",
			usage.input_tokens, usage.cached_input_tokens, usage.output_tokens
		);
		if let Some(post) = last_created {
			cx.caller
				.with_state::<ThreadRecordsQuery, _>(
					move |entity, mut records_mut| {
						records_mut.set_usage(entity, post, usage)
					},
				)
				.await??;
		}
	}
	call_functions(cx.caller, function_calls.into_values()).await?;

	Ok(())
}
