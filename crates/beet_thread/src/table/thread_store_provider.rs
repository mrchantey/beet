use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// The durable, syncable log of a thread's records: [`Thread`], [`Actor`],
/// [`Post`] and [`ResponseMeta`]. Parallels [`BlobStore`] as the source of
/// truth the ephemeral scene is a view onto.
///
/// A thread's records flush into it as they change, each flush a task of its own;
/// [`ThreadStore::flush`] awaits every one, so a thread is torn down only once
/// its records are written.
#[derive(Clone, Deref, Component)]
#[require(SyncedPosts)]
pub struct ThreadStore {
	#[deref]
	provider: Arc<dyn ThreadStoreProvider>,
	/// The flushes spawned and not yet written.
	in_flight: Arc<AtomicUsize>,
}

impl ThreadStore {
	pub fn new(provider: impl ThreadStoreProvider) -> Self {
		Self {
			provider: Arc::new(provider),
			in_flight: default(),
		}
	}
	pub fn inner(&self) -> Arc<dyn ThreadStoreProvider> {
		self.provider.clone()
	}

	/// Write the thread `entity`'s completed posts not yet flushed, then wait
	/// out every flush already in flight, so once this returns the store holds
	/// every completed post of the records. A thread with no store has nothing
	/// to flush.
	pub async fn flush(entity: &AsyncEntity) -> Result {
		let id = entity.id();
		let Some((store, flush)) = entity
			.world()
			.with(move |world: &mut World| {
				let store = world.get::<ThreadStore>(id)?.clone();
				let flush =
					world
						.with_state::<Query<(&Thread, &ThreadRecords, &mut SyncedPosts)>, _>(
							|mut query| {
								query.get_mut(id).ok().and_then(
									|(thread, records, mut synced)| {
										ThreadFlush::take(
											thread,
											records,
											&mut synced,
										)
									},
								)
							},
						);
				(store, flush).xsome()
			})
			.await
		else {
			return Ok(());
		};
		if let Some(flush) = flush {
			flush.write(store.inner()).await?;
		}
		while store.in_flight.load(Ordering::SeqCst) > 0 {
			async_ext::yield_now().await;
			entity.world().with(|_| ()).await;
		}
		Ok(())
	}

	/// Spawn `flush` as a task of its own, counted until it is written.
	pub(crate) fn spawn_flush(
		&self,
		commands: &AsyncCommands,
		flush: ThreadFlush,
	) {
		let provider = self.inner();
		let in_flight = InFlight::new(self.in_flight.clone());
		commands.run(async move |_: AsyncWorld| -> Result {
			let _in_flight = in_flight;
			flush.write(provider).await
		});
	}
}

/// One counted in-flight flush, released however its task ends.
struct InFlight(Arc<AtomicUsize>);

impl InFlight {
	fn new(count: Arc<AtomicUsize>) -> Self {
		count.fetch_add(1, Ordering::SeqCst);
		Self(count)
	}
}

impl Drop for InFlight {
	fn drop(&mut self) { self.0.fetch_sub(1, Ordering::SeqCst); }
}

/// The records a thread holds that its store does not yet: its own completed
/// posts not in [`SyncedPosts`], their authors and response metadata. A
/// branch's inherited posts are its parent's, never written again.
pub(crate) struct ThreadFlush {
	thread: Thread,
	actors: Vec<Actor>,
	posts: Vec<Post>,
	metas: Vec<ResponseMeta>,
}

impl ThreadFlush {
	/// The unflushed completed posts of the records, marking them synced, or `None`
	/// when every completed post is already flushed. In-progress (streaming)
	/// posts stay records-local until completion.
	pub(crate) fn take(
		thread: &Thread,
		records: &ThreadRecords,
		synced: &mut SyncedPosts,
	) -> Option<Self> {
		let posts = records
			.posts()
			.iter()
			.filter(|post| {
				post.thread() == thread.id()
					&& !post.in_progress()
					&& !synced.contains(post.id())
			})
			.cloned()
			.collect::<Vec<_>>();
		if posts.is_empty() {
			return None;
		}
		// dedup the authors and collect metadata for the new posts
		let mut actors: Vec<Actor> = Vec::new();
		let mut metas: Vec<ResponseMeta> = Vec::new();
		for post in &posts {
			if let Ok(actor) = records.actor(post.author())
				&& !actors.iter().any(|existing| existing.id() == actor.id())
			{
				actors.push(actor.clone());
			}
			if let Some(meta) = records.meta(post.id()) {
				metas.push(meta.clone());
			}
		}
		for post in &posts {
			synced.insert(post.id());
		}
		Self {
			thread: thread.clone(),
			actors,
			posts,
			metas,
		}
		.xsome()
	}

	/// Write the records, the thread record first: idempotent, keyed by id.
	async fn write(self, store: Arc<dyn ThreadStoreProvider>) -> Result {
		store.insert_thread(self.thread).await?;
		for actor in self.actors {
			store.insert_actor(actor).await?;
		}
		store.insert_posts(self.posts).await?;
		if !self.metas.is_empty() {
			store.insert_response_metas(self.metas).await?;
		}
		Ok(())
	}
}

impl Default for ThreadStore {
	fn default() -> Self { Self::new(BlobThreadStore::temp()) }
}

/// Backend seam for storing a thread's records. The thread-aware queries
/// ([`Self::thread_posts`] cursor paging, [`Self::full_thread_posts`] join,
/// [`Self::stored_response_meta`]) stay on the trait so a future SQL or
/// automerge backend can run indexed queries where the blob layer can only
/// list-and-scan.
pub trait ThreadStoreProvider: 'static + Send + Sync {
	/// Ensure the backing store(s) exist, creating if needed.
	fn store_try_create(&self) -> BoxedFuture<'_, Result> {
		Box::pin(async move { Ok(()) })
	}
	/// Remove the backing store(s), ignoring any that do not exist.
	fn store_remove(&self) -> BoxedFuture<'_, Result> {
		Box::pin(async move { Ok(()) })
	}

	/// All stored threads. Used on load to discover whether a store already
	/// holds a conversation (and which thread to hydrate).
	fn threads(&self) -> BoxedFuture<'_, Result<Vec<Thread>>>;
	/// All stored actors. Used on load to bind the authored actor scene to stored
	/// actor identities by name.
	fn actors(&self) -> BoxedFuture<'_, Result<Vec<Actor>>>;

	/// Searches the thread for the most recent post with a [`ResponseMeta`]
	/// stored by the provider, for use with `previous_response_id` patterns.
	///
	/// The provider and model slugs are also checked to ensure we get the most
	/// recent meta *for this match*, supporting multi-agent and model-switching
	/// scenarios.
	fn stored_response_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ResponseMeta>>>;
	/// All posts in the thread ordered by [`PostId`], optionally after a cursor.
	fn thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<Post>>>;
	/// As [`Self::thread_posts`], joined with each post's authoring [`Actor`].
	fn full_thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<(Post, Actor)>>>;

	/// The stored thread `id`, if any. Indexed backends override this; the
	/// default scans [`Self::threads`].
	fn thread(&self, id: ThreadId) -> BoxedFuture<'_, Result<Option<Thread>>> {
		Box::pin(async move {
			self.threads()
				.await?
				.into_iter()
				.find(|thread| thread.id() == id)
				.xok()
		})
	}

	/// Every post of `thread`'s history in [`PostId`] order: for a branch, its
	/// parent's history up to and including the post it branches from,
	/// recursively, then its own. A branch references its parent's posts,
	/// never copies them.
	///
	/// ## Errors
	///
	/// Errors when a branch names a thread the store lacks, or the branches
	/// form a cycle.
	fn history<'a>(
		&'a self,
		thread: &'a Thread,
	) -> BoxedFuture<'a, Result<Vec<Post>>> {
		Box::pin(async move {
			let mut posts = self.thread_posts(thread.id(), None).await?;
			let mut visited = vec![thread.id()];
			let mut branch = thread.branch_of().cloned();
			while let Some(point) = branch {
				if visited.contains(&point.thread()) {
					bevybail!("thread {} branches in a cycle", thread.id());
				}
				visited.push(point.thread());
				let parent =
					self.thread(point.thread()).await?.ok_or_else(|| {
						bevyhow!(
							"thread {} branches from {point}, whose thread the store lacks",
							thread.id()
						)
					})?;
				posts.extend(
					self.thread_posts(parent.id(), None)
						.await?
						.into_iter()
						.filter(|post| post.id() <= point.post()),
				);
				branch = parent.branch_of().cloned();
			}
			posts.sort_by_key(|post| post.id());
			posts.xok()
		})
	}

	/// The records `thread` enters with: every actor and its whole
	/// [`Self::history`].
	fn load_records<'a>(
		&'a self,
		thread: &'a Thread,
	) -> BoxedFuture<'a, Result<ThreadRecords>> {
		Box::pin(async move {
			let mut records = ThreadRecords::new();
			for actor in self.actors().await? {
				records.insert_actor(actor);
			}
			for post in self.history(thread).await? {
				records.upsert_post(post);
			}
			records.xok()
		})
	}

	fn insert_actor(&self, actor: Actor) -> BoxedFuture<'_, Result<ActorId>>;
	fn insert_thread(
		&self,
		thread: Thread,
	) -> BoxedFuture<'_, Result<ThreadId>>;
	fn insert_posts(&self, posts: Vec<Post>) -> BoxedFuture<'_, Result<()>>;
	fn insert_response_metas(
		&self,
		metas: Vec<ResponseMeta>,
	) -> BoxedFuture<'_, Result<()>>;
}

impl ThreadStoreProvider for Arc<dyn ThreadStoreProvider> {
	fn store_try_create(&self) -> BoxedFuture<'_, Result> {
		self.as_ref().store_try_create()
	}
	fn store_remove(&self) -> BoxedFuture<'_, Result> {
		self.as_ref().store_remove()
	}
	fn threads(&self) -> BoxedFuture<'_, Result<Vec<Thread>>> {
		self.as_ref().threads()
	}
	fn actors(&self) -> BoxedFuture<'_, Result<Vec<Actor>>> {
		self.as_ref().actors()
	}
	fn stored_response_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ResponseMeta>>> {
		self.as_ref()
			.stored_response_meta(provider_slug, model_slug, thread_id)
	}
	fn thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<Post>>> {
		self.as_ref().thread_posts(thread_id, after_post)
	}
	fn full_thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<(Post, Actor)>>> {
		self.as_ref().full_thread_posts(thread_id, after_post)
	}
	fn thread(&self, id: ThreadId) -> BoxedFuture<'_, Result<Option<Thread>>> {
		self.as_ref().thread(id)
	}
	fn history<'a>(
		&'a self,
		thread: &'a Thread,
	) -> BoxedFuture<'a, Result<Vec<Post>>> {
		self.as_ref().history(thread)
	}
	fn load_records<'a>(
		&'a self,
		thread: &'a Thread,
	) -> BoxedFuture<'a, Result<ThreadRecords>> {
		self.as_ref().load_records(thread)
	}
	fn insert_actor(&self, actor: Actor) -> BoxedFuture<'_, Result<ActorId>> {
		self.as_ref().insert_actor(actor)
	}
	fn insert_thread(
		&self,
		thread: Thread,
	) -> BoxedFuture<'_, Result<ThreadId>> {
		self.as_ref().insert_thread(thread)
	}
	fn insert_posts(&self, posts: Vec<Post>) -> BoxedFuture<'_, Result<()>> {
		self.as_ref().insert_posts(posts)
	}
	fn insert_response_metas(
		&self,
		metas: Vec<ResponseMeta>,
	) -> BoxedFuture<'_, Result<()>> {
		self.as_ref().insert_response_metas(metas)
	}
}


/// Shared conformance harness, mirroring `store_test::run`/`table_test::run`.
/// Invoke once per backend so every future [`ThreadStoreProvider`] is verified
/// in one line.
#[cfg(test)]
pub mod thread_store_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Exercises the full [`ThreadStoreProvider`] surface: insert, id-ordered
	/// reads, cursor paging, the actor join, and stored-response lookup.
	pub async fn run(provider: impl ThreadStoreProvider) {
		provider.store_remove().await.ok();
		provider.store_try_create().await.unwrap();

		// a thread, an actor, and three posts authored by that actor
		let thread = Thread::new("Conformance Thread");
		let thread_id = thread.id();
		provider.insert_thread(thread).await.unwrap();

		let actor = Actor::user();
		let actor_id = provider.insert_actor(actor).await.unwrap();

		let posts: Vec<Post> = (0..3)
			.map(|i| {
				AgentPost::new_text(
					actor_id,
					thread_id,
					format!("post {i}"),
					PostStatus::Completed,
				)
			})
			.collect();
		provider.insert_posts(posts.clone()).await.unwrap();

		// reads come back id-sorted, independent of insertion order
		let mut expected = posts.clone();
		expected.sort_by_key(|post| post.id());
		let stored = provider.thread_posts(thread_id, None).await.unwrap();
		stored.len().xpect_eq(3);
		stored
			.iter()
			.map(|post| post.id())
			.collect::<Vec<_>>()
			.xpect_eq(
				expected.iter().map(|post| post.id()).collect::<Vec<_>>(),
			);

		// posts from another thread are excluded
		provider
			.thread_posts(ThreadId::new_now(), None)
			.await
			.unwrap()
			.len()
			.xpect_eq(0);

		// the cursor returns only posts after the given id (exclusive)
		let after = provider
			.thread_posts(thread_id, Some(expected[0].id()))
			.await
			.unwrap();
		after
			.iter()
			.map(|post| post.id())
			.collect::<Vec<_>>()
			.xpect_eq(
				expected[1..]
					.iter()
					.map(|post| post.id())
					.collect::<Vec<_>>(),
			);

		// the actor join resolves each post's author
		let full = provider.full_thread_posts(thread_id, None).await.unwrap();
		full.len().xpect_eq(3);
		full.iter()
			.all(|(post, actor)| post.author() == actor.id())
			.xpect_true();

		// stored-response lookup matches provider+model and ignores unstored
		provider
			.insert_response_metas(vec![ResponseMeta {
				post_id: expected[2].id(),
				provider_slug: "openai".into(),
				model_slug: "gpt-5-mini".into(),
				response_id: "resp_1".into(),
				response_stored: true,
				usage: None,
			}])
			.await
			.unwrap();
		provider
			.stored_response_meta("openai", "gpt-5-mini", thread_id)
			.await
			.unwrap()
			.unwrap()
			.post_id
			.xpect_eq(expected[2].id());
		provider
			.stored_response_meta("anthropic", "claude", thread_id)
			.await
			.unwrap()
			.xpect_none();

		// a branch of the second post holds the parent's history up to it,
		// then its own, and a sibling branch of that post is independent
		let point = PostRef::new(thread_id, expected[1].id());
		let branch = |text: &'static str| {
			let branch = Thread::new("Branch").with_branch_of(point.clone());
			let post = AgentPost::new_text(
				actor_id,
				branch.id(),
				text,
				PostStatus::Completed,
			);
			(branch, post)
		};
		let (left, left_post) = branch("left");
		let (right, right_post) = branch("right");
		for (branch, post) in [(&left, left_post), (&right, right_post)] {
			provider.insert_thread(branch.clone()).await.unwrap();
			provider.insert_posts(vec![post]).await.unwrap();
		}
		let texts = |posts: Vec<Post>| {
			posts
				.iter()
				.map(|post| post.to_string())
				.collect::<Vec<_>>()
		};
		texts(provider.history(&left).await.unwrap())
			.xpect_eq(vec!["post 0", "post 1", "left"]);
		texts(provider.history(&right).await.unwrap())
			.xpect_eq(vec!["post 0", "post 1", "right"]);
		provider
			.load_records(&left)
			.await
			.unwrap()
			.posts()
			.iter()
			.map(|post| post.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["post 0", "post 1", "left"]);

		provider.store_remove().await.unwrap();
	}

	#[beet_core::test]
	async fn memory() { run(BlobThreadStore::temp()).await; }

	// full-store conformance over real disk: a dozen blob read-modify-writes,
	// which can exceed the 5s default under parallel test load.
	#[beet_core::test(timeout_ms = 30000)]
	async fn fs() {
		use beet_net::prelude::*;
		let dir = TempDir::new_ws().unwrap();
		let blob = BlobStore::new(FsStore::new(dir.path().clone()));
		run(BlobThreadStore::new(blob)).await;
	}
}
