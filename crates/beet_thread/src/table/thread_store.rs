use crate::prelude::*;
use beet_core::prelude::*;

/// Tracks which window posts have already been flushed to the [`ThreadStore`],
/// so a coarse `Changed<ThreadWindow>` only commits newly-completed posts.
/// Loaded history is pre-marked, so it is never re-written.
#[derive(Debug, Default, Clone, Component)]
pub struct SyncedPosts(HashSet<PostId>);

impl SyncedPosts {
	pub fn new(synced: HashSet<PostId>) -> Self { Self(synced) }
	pub fn contains(&self, id: PostId) -> bool { self.0.contains(&id) }
	pub fn insert(&mut self, id: PostId) { self.0.insert(id); }
}

/// Flush a thread's window to its [`ThreadStore`] whenever the window changes.
///
/// The window is the single in-memory truth the scene streams into; this is the
/// one channel that persists it. Only completed (`!in_progress`) posts not yet
/// in [`SyncedPosts`] are committed, with their authoring actors and any
/// response metadata; the thread record is idempotent (keyed by id). In-progress
/// (streaming) posts stay window-local until completion. Threads without a
/// [`ThreadStore`] are ignored.
pub fn sync_window_to_store(
	async_commands: AsyncCommands,
	mut changed: Query<
		(&Thread, &ThreadWindow, &ThreadStore, &mut SyncedPosts),
		Changed<ThreadWindow>,
	>,
) -> Result {
	for (thread, window, store, mut synced) in changed.iter_mut() {
		// completed posts not yet flushed
		let new_posts = window
			.posts()
			.iter()
			.filter(|post| !post.in_progress() && !synced.contains(post.id()))
			.cloned()
			.collect::<Vec<_>>();
		if new_posts.is_empty() {
			continue;
		}

		// dedup the authors and collect metadata for the new posts
		let mut actors: Vec<Actor> = Vec::new();
		let mut metas: Vec<ResponseMeta> = Vec::new();
		for post in &new_posts {
			if let Ok(actor) = window.actor(post.author())
				&& !actors.iter().any(|existing| existing.id() == actor.id())
			{
				actors.push(actor.clone());
			}
			if let Some(meta) = window.meta(post.id()) {
				metas.push(meta.clone());
			}
		}
		new_posts.iter().for_each(|post| synced.insert(post.id()));

		let store = store.inner();
		let thread = thread.clone();
		async_commands.run(async move |_: AsyncWorld| -> Result {
			store.insert_thread(thread).await?;
			for actor in actors {
				store.insert_actor(actor).await?;
			}
			store.insert_posts(new_posts).await?;
			if !metas.is_empty() {
				store.insert_response_metas(metas).await?;
			}
			Ok(())
		});
	}

	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	/// A completed post lands in the thread's [`ThreadStore`] as a record.
	#[beet_core::test]
	async fn syncs_completed_post_to_store() {
		let backing = BlobThreadStore::temp();
		let store = ThreadStore::new(backing.clone());

		let mut app = App::new();
		app.add_plugins(MinimalPlugins).init_plugin::<ThreadPlugin>();

		let thread = app
			.world_mut()
			.spawn((
				Thread::default(),
				store.clone(),
				children![
					(Actor::user(), children![Post::spawn("hi")]),
					(Actor::agent(), MockPostStreamer::default()),
				],
			))
			.id();
		app.update();

		// run the agent so it replies into the window
		let agent = app
			.world_mut()
			.with_state::<Query<Entity, (With<ActorRef>, With<Action<(), Outcome>>)>, _>(
				|query| query.iter().next(),
			)
			.unwrap();
		app.world_mut().entity_mut(agent).call::<(), Outcome>(()).await.unwrap();

		// pump so the sync system runs and its task flushes
		for _ in 0..20 {
			app.update();
		}

		let thread_id = app.world().get::<Thread>(thread).unwrap().id();
		let posts = backing.thread_posts(thread_id, None).await.unwrap();
		posts
			.iter()
			.any(|post| post.body_str().ok() == Some("you said: hi"))
			.xpect_true();
	}
}
