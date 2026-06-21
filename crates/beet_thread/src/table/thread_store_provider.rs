use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::sync::Arc;

/// The durable, syncable log of a thread's records: [`Thread`], [`Actor`],
/// [`Post`] and [`ResponseMeta`]. Parallels [`BlobStore`] as the source of
/// truth the ephemeral scene is a view onto.
#[derive(Clone, Deref, Component)]
#[require(SyncedPosts)]
pub struct ThreadStore(Arc<dyn ThreadStoreProvider>);

impl ThreadStore {
	pub fn new(provider: impl ThreadStoreProvider) -> Self {
		Self(Arc::new(provider))
	}
	pub fn inner(&self) -> Arc<dyn ThreadStoreProvider> { self.0.clone() }
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

	/// Materialize a [`ThreadWindow`] for a thread (optionally a tail after a
	/// cursor): the read side of the window/sync seam. Indexed backends override
	/// this; the default joins [`Self::actors`] with [`Self::thread_posts`].
	fn materialize_window(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<ThreadWindow>> {
		Box::pin(async move {
			let mut window = ThreadWindow::new();
			self.actors().await?.into_iter().for_each(|actor| {
				window.insert_actor(actor);
			});
			self.thread_posts(thread_id, after_post)
				.await?
				.into_iter()
				.for_each(|post| window.upsert_post(post));
			Ok(window)
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

// ═══════════════════════════════════════════════════════════════════════
// TableStoreRow bridge
// ═══════════════════════════════════════════════════════════════════════

/// Transparent storage wrapper bridging a record to [`TableStoreRow`].
///
/// The records key on a typed [`Uuid7`] via the local [`Table`] trait;
/// [`TableStoreRow`] keys on the raw [`uuid::Uuid`]. Implementing the latter
/// directly on the records would make every `record.id()` call ambiguous
/// between the two traits, so the bridge lives on this private newtype
/// instead. `#[serde(transparent)]` keeps the on-disk bytes identical to the
/// bare record.
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
struct StoredRow<T>(T);

impl TableStoreRow for StoredRow<Thread> {
	fn id(&self) -> uuid::Uuid { self.0.id().uuid() }
}
impl TableStoreRow for StoredRow<Actor> {
	fn id(&self) -> uuid::Uuid { self.0.id().uuid() }
}
impl TableStoreRow for StoredRow<Post> {
	fn id(&self) -> uuid::Uuid { self.0.id().uuid() }
}
impl TableStoreRow for StoredRow<ResponseMeta> {
	fn id(&self) -> uuid::Uuid { self.0.id().uuid() }
}

// ═══════════════════════════════════════════════════════════════════════
// BlobThreadStore
// ═══════════════════════════════════════════════════════════════════════

/// The generic blob-backed [`ThreadStoreProvider`]: one [`TableStore`] per
/// record type over subdirs of a shared [`BlobStore`]. Every blob backend
/// (memory, fs, s3, ...) yields a thread store for free; the thread queries
/// are implemented as list-and-scan, sufficient for the hot tier and moderate
/// threads. Specialized indexed backends implement [`ThreadStoreProvider`]
/// directly.
#[derive(Clone)]
pub struct BlobThreadStore {
	threads: TableStore<StoredRow<Thread>>,
	actors: TableStore<StoredRow<Actor>>,
	posts: TableStore<StoredRow<Post>>,
	metas: TableStore<StoredRow<ResponseMeta>>,
}

impl BlobThreadStore {
	/// Back a thread store with the given [`BlobStore`], partitioning record
	/// types into `threads/`, `actors/`, `posts/` and `metas/` subdirs.
	pub fn new(blob: BlobStore) -> Self {
		Self {
			threads: TableStore::new(
				blob.with_subdir(SmolPath::new("threads")),
			),
			actors: TableStore::new(blob.with_subdir(SmolPath::new("actors"))),
			posts: TableStore::new(blob.with_subdir(SmolPath::new("posts"))),
			metas: TableStore::new(blob.with_subdir(SmolPath::new("metas"))),
		}
	}
	/// An in-memory thread store, ie for tests and the hot window.
	pub fn temp() -> Self { Self::new(BlobStore::temp()) }
}

/// Apply an exclusive `after` cursor to id-sorted posts. A missing cursor
/// returns the full list; an unknown cursor returns the full list (the caller
/// has not yet seen any of these posts).
fn after_cursor(posts: Vec<Post>, after_post: Option<PostId>) -> Vec<Post> {
	let Some(after) = after_post else {
		return posts;
	};
	match posts.iter().position(|post| post.id() == after) {
		Some(i) => posts[i + 1..].to_vec(),
		None => posts,
	}
}

impl ThreadStoreProvider for BlobThreadStore {
	fn store_try_create(&self) -> BoxedFuture<'_, Result> {
		Box::pin(async move {
			self.threads.store_try_create().await?;
			self.actors.store_try_create().await?;
			self.posts.store_try_create().await?;
			self.metas.store_try_create().await?;
			Ok(())
		})
	}

	fn store_remove(&self) -> BoxedFuture<'_, Result> {
		Box::pin(async move {
			// ignore stores that were never created
			self.threads.store_remove().await.ok();
			self.actors.store_remove().await.ok();
			self.posts.store_remove().await.ok();
			self.metas.store_remove().await.ok();
			Ok(())
		})
	}

	fn threads(&self) -> BoxedFuture<'_, Result<Vec<Thread>>> {
		Box::pin(async move {
			self.threads
				.get_all()
				.await?
				.into_iter()
				.map(|(_, thread)| thread.0)
				.collect::<Vec<_>>()
				.xok()
		})
	}

	fn actors(&self) -> BoxedFuture<'_, Result<Vec<Actor>>> {
		Box::pin(async move {
			self.actors
				.get_all()
				.await?
				.into_iter()
				.map(|(_, actor)| actor.0)
				.collect::<Vec<_>>()
				.xok()
		})
	}

	fn stored_response_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ResponseMeta>>> {
		Box::pin(async move {
			let posts = self.thread_posts(thread_id, None).await?;
			let metas: Vec<ResponseMeta> = self
				.metas
				.get_all()
				.await?
				.into_iter()
				.map(|(_, meta)| meta.0)
				.collect();
			// most recent (posts are id-sorted) stored meta for this match
			posts
				.iter()
				.filter_map(|post| {
					metas.iter().find(|meta| {
						meta.post_id == post.id()
							&& meta.provider_slug == provider_slug
							&& meta.model_slug == model_slug
							&& meta.response_stored
					})
				})
				.last()
				.cloned()
				.xok()
		})
	}

	fn thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<Post>>> {
		Box::pin(async move {
			let mut posts: Vec<Post> = self
				.posts
				.get_all()
				.await?
				.into_iter()
				.map(|(_, post)| post.0)
				.filter(|post| post.thread() == thread_id)
				.collect();
			posts.sort_by_key(|post| post.id());
			after_cursor(posts, after_post).xok()
		})
	}

	fn full_thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<(Post, Actor)>>> {
		Box::pin(async move {
			let posts = self.thread_posts(thread_id, after_post).await?;
			let mut out = Vec::with_capacity(posts.len());
			for post in posts {
				let actor = self.actors.get(post.author().uuid()).await?.0;
				out.push((post, actor));
			}
			out.xok()
		})
	}

	fn insert_actor(&self, actor: Actor) -> BoxedFuture<'_, Result<ActorId>> {
		Box::pin(async move {
			let id = actor.id();
			self.actors.push(StoredRow(actor)).await?;
			Ok(id)
		})
	}

	fn insert_thread(
		&self,
		thread: Thread,
	) -> BoxedFuture<'_, Result<ThreadId>> {
		Box::pin(async move {
			let id = thread.id();
			self.threads.push(StoredRow(thread)).await?;
			Ok(id)
		})
	}

	fn insert_posts(&self, posts: Vec<Post>) -> BoxedFuture<'_, Result<()>> {
		Box::pin(async move {
			for post in posts {
				self.posts.push(StoredRow(post)).await?;
			}
			Ok(())
		})
	}

	fn insert_response_metas(
		&self,
		metas: Vec<ResponseMeta>,
	) -> BoxedFuture<'_, Result<()>> {
		Box::pin(async move {
			for meta in metas {
				self.metas.push(StoredRow(meta)).await?;
			}
			Ok(())
		})
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

		provider.store_remove().await.unwrap();
	}

	#[beet_core::test]
	async fn memory() { run(BlobThreadStore::temp()).await; }

	#[beet_core::test]
	async fn fs() {
		use beet_net::prelude::*;
		let blob = BlobStore::new(FsStore::new(
			AbsPathBuf::new_workspace_rel(
				"target/tests/beet_thread/thread-store-fs",
			)
			.unwrap(),
		));
		run(BlobThreadStore::new(blob)).await;
	}
}
