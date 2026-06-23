use crate::prelude::*;
// `Table::id()` on the records; the `beet_net` glob otherwise shadows it.
use crate::table::Table;
use async_lock::Mutex;
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::sync::Arc;

/// The generic blob-backed [`ThreadStoreProvider`]: a whole thread (its
/// [`Thread`], [`Actor`]s, [`Post`]s and [`ResponseMeta`]) persisted as a single
/// JSON object in one [`BlobStore`] slot.
///
/// Loading a conversation is one read and saving is one write; a blob-per-record
/// layout instead fans out to a read (and a write) per record, hundreds for a
/// long thread. A loaded snapshot is held in memory (read once, lazily) and every
/// write rewrites the one object under an async lock, so concurrent flushes never
/// lose an update. Records are keyed by id (upsert), so re-storing a thread or its
/// posts on reload never duplicates them. Every blob backend (memory, fs, s3)
/// yields a thread store for free; the thread queries scan the in-memory snapshot.
/// Specialized indexed backends implement [`ThreadStoreProvider`] directly.
#[derive(Clone)]
pub struct BlobThreadStore {
	store: BlobStore,
	snapshot: Arc<Mutex<Option<ThreadData>>>,
}

/// The persisted document: every record type as a list, one JSON object.
#[derive(Default, Clone, Serialize, Deserialize)]
struct ThreadData {
	threads: Vec<Thread>,
	actors: Vec<Actor>,
	posts: Vec<Post>,
	metas: Vec<ResponseMeta>,
}

/// The single object, under the store's root, every record persists into.
const STORE_PATH: &str = "thread.json";

impl BlobThreadStore {
	/// Back a thread store with `blob`, persisting all records to one JSON object.
	pub fn new(blob: BlobStore) -> Self {
		Self {
			store: blob,
			snapshot: Arc::new(Mutex::new(None)),
		}
	}
	/// An in-memory thread store, ie for tests and the hot window.
	pub fn temp() -> Self { Self::new(BlobStore::temp()) }

	/// Read the persisted snapshot, loading it once into memory.
	async fn read(&self) -> Result<ThreadData> {
		let mut guard = self.snapshot.lock().await;
		ensure_loaded(&self.store, &mut guard).await?;
		Ok(guard.as_ref().unwrap().clone())
	}

	/// Mutate the snapshot under the lock, then persist the whole document.
	async fn write(&self, mutate: impl FnOnce(&mut ThreadData)) -> Result {
		let mut guard = self.snapshot.lock().await;
		ensure_loaded(&self.store, &mut guard).await?;
		mutate(guard.as_mut().unwrap());
		let bytes = serde_json::to_vec(guard.as_ref().unwrap())?;
		self.store.insert(&SmolPath::new(STORE_PATH), bytes).await
	}
}

/// Load the document into `guard` once, defaulting to empty when the object is
/// absent (a fresh store).
async fn ensure_loaded(
	store: &BlobStore,
	guard: &mut Option<ThreadData>,
) -> Result {
	if guard.is_none() {
		let path = SmolPath::new(STORE_PATH);
		let data = match store.exists(&path).await? {
			true => serde_json::from_slice(&store.get(&path).await?)?,
			false => ThreadData::default(),
		};
		*guard = Some(data);
	}
	Ok(())
}

/// Replace the entry matching `item`'s key, or append it: keeps each record id
/// unique so reloading a thread never duplicates its records.
fn upsert<T, K: PartialEq>(list: &mut Vec<T>, item: T, key: impl Fn(&T) -> K) {
	let item_key = key(&item);
	match list.iter_mut().find(|existing| key(existing) == item_key) {
		Some(existing) => *existing = item,
		None => list.push(item),
	}
}

/// A thread's posts from the snapshot, id-sorted, after an exclusive cursor. A
/// missing or unknown cursor returns every post (the caller has seen none).
fn thread_posts_of(
	data: &ThreadData,
	thread_id: ThreadId,
	after_post: Option<PostId>,
) -> Vec<Post> {
	let mut posts = data
		.posts
		.iter()
		.filter(|post| post.thread() == thread_id)
		.cloned()
		.collect::<Vec<_>>();
	posts.sort_by_key(|post| post.id());
	let Some(after) = after_post else {
		return posts;
	};
	match posts.iter().position(|post| post.id() == after) {
		Some(index) => posts[index + 1..].to_vec(),
		None => posts,
	}
}

impl ThreadStoreProvider for BlobThreadStore {
	fn store_try_create(&self) -> BoxedFuture<'_, Result> {
		Box::pin(async move {
			// idempotent: some backends (the in-memory store) error if recreated
			if !self.store.store_exists().await? {
				self.store.store_create().await?;
			}
			Ok(())
		})
	}

	fn store_remove(&self) -> BoxedFuture<'_, Result> {
		Box::pin(async move {
			// drop the one object and reset the in-memory snapshot to empty
			self.store.remove(&SmolPath::new(STORE_PATH)).await.ok();
			*self.snapshot.lock().await = Some(ThreadData::default());
			Ok(())
		})
	}

	fn threads(&self) -> BoxedFuture<'_, Result<Vec<Thread>>> {
		Box::pin(async move { self.read().await?.threads.xok() })
	}

	fn actors(&self) -> BoxedFuture<'_, Result<Vec<Actor>>> {
		Box::pin(async move { self.read().await?.actors.xok() })
	}

	fn stored_response_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ResponseMeta>>> {
		Box::pin(async move {
			let data = self.read().await?;
			let posts = thread_posts_of(&data, thread_id, None);
			// most recent (posts are id-sorted) stored meta for this match
			posts
				.iter()
				.filter_map(|post| {
					data.metas.iter().find(|meta| {
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
			thread_posts_of(&self.read().await?, thread_id, after_post).xok()
		})
	}

	fn full_thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<(Post, Actor)>>> {
		Box::pin(async move {
			let data = self.read().await?;
			thread_posts_of(&data, thread_id, after_post)
				.into_iter()
				.map(|post| {
					let post_id = post.id();
					data.actors
						.iter()
						.find(|actor| actor.id() == post.author())
						.cloned()
						.map(|actor| (post, actor))
						.ok_or_else(|| bevyhow!("no actor for post {post_id}"))
				})
				.collect()
		})
	}

	fn insert_actor(&self, actor: Actor) -> BoxedFuture<'_, Result<ActorId>> {
		Box::pin(async move {
			let id = actor.id();
			self.write(move |data| upsert(&mut data.actors, actor, Actor::id))
				.await?;
			Ok(id)
		})
	}

	fn insert_thread(
		&self,
		thread: Thread,
	) -> BoxedFuture<'_, Result<ThreadId>> {
		Box::pin(async move {
			let id = thread.id();
			self.write(move |data| {
				upsert(&mut data.threads, thread, Thread::id)
			})
			.await?;
			Ok(id)
		})
	}

	fn insert_posts(&self, posts: Vec<Post>) -> BoxedFuture<'_, Result<()>> {
		Box::pin(async move {
			self.write(move |data| {
				posts
					.into_iter()
					.for_each(|post| upsert(&mut data.posts, post, Post::id));
			})
			.await
		})
	}

	fn insert_response_metas(
		&self,
		metas: Vec<ResponseMeta>,
	) -> BoxedFuture<'_, Result<()>> {
		Box::pin(async move {
			self.write(move |data| {
				// one meta per post: keyed by post id
				metas.into_iter().for_each(|meta| {
					upsert(&mut data.metas, meta, |meta| meta.post_id)
				});
			})
			.await
		})
	}
}
