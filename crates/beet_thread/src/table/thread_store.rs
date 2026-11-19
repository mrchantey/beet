use crate::prelude::*;
use beet_core::prelude::*;

/// Tracks which of a thread's posts have already been flushed to the [`ThreadStore`],
/// so a coarse `Changed<ThreadRecords>` only commits newly-completed posts.
/// Loaded history is pre-marked, so it is never re-written.
#[derive(Debug, Default, Clone, Component)]
pub struct SyncedPosts(HashSet<PostId>);

impl SyncedPosts {
	pub fn new(synced: HashSet<PostId>) -> Self { Self(synced) }
	pub fn contains(&self, id: PostId) -> bool { self.0.contains(&id) }
	pub fn insert(&mut self, id: PostId) { self.0.insert(id); }
	/// Mark `id` unflushed, so the next flush writes it again.
	pub fn remove(&mut self, id: PostId) { self.0.remove(&id); }
}

/// Flush a thread's records to its [`ThreadStore`] whenever its records change.
///
/// The records are the single in-memory truth the scene streams into; this is the
/// one channel that persists it. Only completed (`!in_progress`) posts not yet
/// in [`SyncedPosts`] are committed, with their authoring actors and any
/// response metadata ([`ThreadFlush`]); the thread record is idempotent (keyed
/// by id). Each flush is a task counted on its store, which
/// [`ThreadStore::flush`] awaits. Threads without a [`ThreadStore`] are
/// ignored.
pub(crate) fn sync_records_to_store(
	async_commands: AsyncCommands,
	mut changed: Query<
		(&Thread, &ThreadRecords, &ThreadStore, &mut SyncedPosts),
		Changed<ThreadRecords>,
	>,
) -> Result {
	for (thread, records, store, mut synced) in changed.iter_mut() {
		if let Some(flush) = ThreadFlush::take(thread, records, &mut synced) {
			store.spawn_flush(&async_commands, flush);
		}
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
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();

		let thread = app
			.world_mut()
			.spawn((Thread::default(), store.clone(), children![
				(Actor::user(), children![Post::spawn("hi")]),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		app.update();

		// run the agent so it replies into the records
		let agent = app
			.world_mut()
			.with_state::<Query<Entity, (With<ActorRef>, With<Action<(), Outcome>>)>, _>(
				|query| query.iter().next(),
			)
			.unwrap();
		app.world_mut()
			.entity_mut(agent)
			.call::<(), Outcome>(())
			.await
			.unwrap();

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
