use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Spawn a single-turn thread from a declarative actor `roster`, run its agent
/// to completion, and collect the posts it authored.
///
/// The roster is the thread's actors and their seed posts; exactly one actor
/// must carry a streamer [`Action`] (the agent). The scene reduces into a
/// [`ThreadWindow`]; once the agent responds, the posts appended after the seed
/// window are returned, ie the agent's reply.
///
/// This is the non-interactive counterpart to a long-running server scene: the
/// `oneshot` example and the [`PostStreamer`] conformance tests build on it.
pub async fn run_oneshot(roster: impl Bundle) -> Result<Vec<Post>> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins).init_plugin::<ThreadPlugin>();
	let world = app.world_mut();

	let thread = world.spawn((Thread::new("Oneshot Thread"), roster)).id();
	world.flush();
	// reduce the author scene into a ThreadWindow + behavior entities
	reduce_threads_now(world);

	// the agent is the behavior entity carrying a streamer Action
	let agent = world
		.with_state::<Query<Entity, (With<ActorRef>, With<Action<(), Outcome>>)>, _>(
			|query| query.iter().next(),
		)
		.ok_or_else(|| {
			bevyhow!("oneshot roster has no agent actor with a streamer Action")
		})?;

	// a cursor past the seed window, so only the agent's posts come back
	let cursor = world.with_state::<Query<&ThreadWindow>, _>(move |windows| {
		windows
			.get(thread)
			.ok()
			.and_then(|window| window.last_post().map(|post| post.id()))
	});

	world.entity_mut(agent).call::<(), Outcome>(()).await?;

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
