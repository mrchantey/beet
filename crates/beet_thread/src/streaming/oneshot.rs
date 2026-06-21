use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Spawn a single-turn thread from an `author_scene` (its actors and seed posts),
/// run the thread to completion, and collect the posts it authored.
///
/// You **call the thread**, not "the agent": the scene reduces into a
/// [`ThreadWindow`], then calling the thread's `Sequence` runs each actor's turn
/// in document order. Whatever posts land after the seed window are returned, ie
/// the replies. This presumes no topology, so it works for zero, one, or many
/// agents, with or without a user actor.
///
/// The non-interactive counterpart to a long-running server scene: the `oneshot`
/// example and the [`PostStreamer`] conformance tests build on it.
pub async fn run_oneshot(author_scene: impl Bundle) -> Result<Vec<Post>> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.init_plugin::<ThreadPlugin>();
	let world = app.world_mut();

	// the thread is its actors wrapped in a Sequence; a seed-only actor carries
	// no action and is skipped, the rest take their turn in order
	let thread = world
		.spawn((
			Thread::new("Oneshot Thread"),
			Sequence::new(),
			ExcludeErrors(ChildError::NO_ACTION),
			author_scene,
		))
		.id();
	world.flush();
	// reduce the author scene into a ThreadWindow + behavior entities
	ThreadWindow::reduce_now(world);

	// a cursor past the seed window, so only the turn's posts come back
	let cursor = world.with_state::<Query<&ThreadWindow>, _>(move |windows| {
		windows
			.get(thread)
			.ok()
			.and_then(|window| window.last_post().map(|post| post.id()))
	});

	world.entity_mut(thread).call::<(), Outcome>(()).await?;

	world.with_state::<Query<&ThreadWindow>, _>(
		move |windows| -> Result<Vec<Post>> {
			windows
				.get(thread)?
				.posts_after(cursor)
				.into_iter()
				.map(|view| view.post.clone())
				.collect::<Vec<_>>()
				.xok()
		},
	)
}
